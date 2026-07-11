//! Registry-aware strict admission and atomic declaration registration.

use std::sync::Arc;

use async_trait::async_trait;
use selta_core::{
    AdmissionPolicy, BuiltinAdmissionLimits, Determinism, EffectClass, Envelope, ExtensionDecl,
    ExtensionHost, HostCall, InputDomain, MetaIssue, MetaIssueCode, Needs, Node, Registry,
};
use serde_json::{json, Value};

#[derive(Default)]
struct PassHost;

#[async_trait]
impl ExtensionHost for PassHost {
    async fn verify(&self, _call: HostCall<'_>) -> Result<Envelope, String> {
        Ok(Envelope::pass())
    }
}

fn node(value: Value) -> Node {
    Node::from_value(value).expect("test schema projects")
}

fn declaration(name: &str, effect_class: EffectClass) -> ExtensionDecl {
    ExtensionDecl {
        name: name.to_string(),
        semantic_revision: format!("selta.test.{name}.v1"),
        cacheable: effect_class == EffectClass::Pure,
        determinism: Determinism::Deterministic,
        effect_class,
        accepted_input: InputDomain::any(),
        config_schema: None,
        config_preflight: None,
        needs: Needs::default(),
        settings_schema: None,
        delta_schema: None,
    }
}

fn admit_errors(registry: &Registry, value: Value, policy: &AdmissionPolicy) -> Vec<MetaIssue> {
    registry
        .admit_source(&serde_json::to_vec(&value).unwrap(), policy)
        .expect_err("fixture must fail registry-aware admission")
}

fn has(issues: &[MetaIssue], code: MetaIssueCode, pointer: &str) -> bool {
    issues
        .iter()
        .any(|issue| issue.code == code && issue.pointer == pointer)
}

#[test]
fn registry_admission_reports_unknown_effect_input_sampling_and_semantics() {
    let mut registry = Registry::with_pure_builtins();
    registry
        .register(
            vec![declaration("process_probe", EffectClass::ProcessIo)],
            Arc::new(PassHost),
        )
        .unwrap();

    let fixtures = [
        (
            json!({ "type": "str", "verify": [{ "ext": "missing" }] }),
            MetaIssueCode::UnknownExtension,
            "/verify/0/ext",
        ),
        (
            json!({ "type": "str", "verify": [{ "ext": "process_probe" }] }),
            MetaIssueCode::EffectMismatch,
            "/verify/0/ext",
        ),
        (
            json!({ "type": "int", "verify": [{ "ext": "regex", "config": { "pattern": "x" } }] }),
            MetaIssueCode::InputDomainMismatch,
            "/verify/0/ext",
        ),
        (
            json!({ "type": "str", "verify": [{ "ext": "regex", "config": { "pattern": "x" }, "sampling": { "samples": 1 } }] }),
            MetaIssueCode::InvalidSampling,
            "/verify/0/sampling",
        ),
        (
            json!({ "type": "str", "verify": [{ "ext": "regex", "config": { "pattern": "[" } }] }),
            MetaIssueCode::ConfigSemantics,
            "/verify/0/config",
        ),
    ];

    for (source, code, pointer) in fixtures {
        let issues = admit_errors(&registry, source, &AdmissionPolicy::pure_only());
        assert!(has(&issues, code, pointer), "{issues:?}");
    }
}

#[test]
fn exact_env_holes_defer_only_their_slot_and_literal_siblings_still_validate() {
    let registry = Registry::with_pure_builtins();
    registry
        .admit_source(
            &serde_json::to_vec(&json!({
                "type": "str",
                "verify": [{
                    "ext": "regex",
                    "config": { "pattern": { "$env": "request.pattern" } }
                }]
            }))
            .unwrap(),
            &AdmissionPolicy::pure_only(),
        )
        .expect("a well-formed hole defers semantic regex compilation");

    let issues = admit_errors(
        &registry,
        json!({
            "type": "float",
            "verify": [{
                "ext": "range",
                "config": {
                    "min": { "$env": "request.minimum" },
                    "max": "not-a-number"
                }
            }]
        }),
        &AdmissionPolicy::pure_only(),
    );
    assert!(has(
        &issues,
        MetaIssueCode::ConfigStructure,
        "/verify/0/config/max"
    ));
    assert!(!issues
        .iter()
        .any(|issue| issue.pointer == "/verify/0/config/min"));
}

#[test]
fn builtin_resource_ceilings_are_explicit_generic_policy() {
    let registry = Registry::with_pure_builtins();
    let policy = AdmissionPolicy::pure_only()
        .with_builtin_limits(BuiltinAdmissionLimits::new(Some(2), Some(1)));
    let issues = admit_errors(
        &registry,
        json!({
            "type": "str",
            "verify": [
                { "ext": "one_of", "config": { "values": ["a", "b", "c"] } },
                { "ext": "regex", "config": { "pattern": "é" } }
            ]
        }),
        &policy,
    );
    assert!(has(
        &issues,
        MetaIssueCode::ResourceLimitExceeded,
        "/verify/0/config/values"
    ));
    assert!(has(
        &issues,
        MetaIssueCode::ResourceLimitExceeded,
        "/verify/1/config/pattern"
    ));
    assert_eq!(
        MetaIssueCode::ResourceLimitExceeded.as_str(),
        "RESOURCE_LIMIT_EXCEEDED"
    );
}

#[test]
fn registration_rejects_recursive_config_and_settings_verifiers_atomically() {
    for role in ["config", "settings"] {
        let mut registry = Registry::with_pure_builtins();
        let mut decl = declaration(&format!("bad_{role}"), EffectClass::Pure);
        let annotated = node(json!({
            "type": "object",
            "fields": {
                "nested": {
                    "type": "array",
                    "item": {
                        "type": "str",
                        "verify": [{ "ext": "regex", "config": { "pattern": "x" } }]
                    }
                }
            }
        }));
        if role == "config" {
            decl.config_schema = Some(annotated);
        } else {
            decl.settings_schema = Some(annotated);
        }
        let error = registry
            .register(vec![decl], Arc::new(PassHost))
            .expect_err("declaration schemas cannot recursively execute verifiers");
        assert!(error.contains(&format!("{role}_schema")), "{error}");
        assert!(error.contains("/fields/nested/item/verify"), "{error}");
        assert!(registry.decl(&format!("bad_{role}")).is_none());
    }
}

#[test]
fn registration_strict_normalization_rejects_malformed_typed_schemas() {
    let mut registry = Registry::with_pure_builtins();
    let mut decl = declaration("bad_shape", EffectClass::Pure);
    // The compatibility parser can represent this; strict declaration
    // normalization must not let it enter the registry.
    decl.config_schema = Some(node(json!({ "type": "union", "variants": [] })));
    let error = registry
        .register(vec![decl], Arc::new(PassHost))
        .expect_err("empty declaration union is not a strict schema");
    assert!(error.contains("invalid config_schema"), "{error}");
    assert!(error.contains("EMPTY_COLLECTION"), "{error}");
    assert!(registry.decl("bad_shape").is_none());
}

#[test]
fn prospective_registry_allows_batch_references_and_rejects_bad_delta_atomically() {
    let host: Arc<dyn ExtensionHost> = Arc::new(PassHost);

    let mut registry = Registry::with_pure_builtins();
    let mut producer = declaration("producer", EffectClass::Pure);
    producer.delta_schema = Some(node(json!({
        "type": "str",
        "verify": [{ "ext": "batch_peer" }]
    })));
    let peer = declaration("batch_peer", EffectClass::Pure);
    registry
        .register(vec![producer, peer], host.clone())
        .expect("a delta schema may reference another declaration in its atomic batch");
    assert!(registry.decl("producer").is_some());
    assert!(registry.decl("batch_peer").is_some());

    let mut bad = declaration("bad_producer", EffectClass::Pure);
    bad.delta_schema = Some(node(json!({
        "type": "str",
        "verify": [{ "ext": "not_in_prospective_registry" }]
    })));
    let innocent = declaration("innocent_peer", EffectClass::Pure);
    let error = registry
        .register(vec![bad, innocent], host)
        .expect_err("unknown delta verifier rejects the entire batch");
    assert!(error.contains("UNKNOWN_EXTENSION"), "{error}");
    assert!(registry.decl("bad_producer").is_none());
    assert!(registry.decl("innocent_peer").is_none());
}

#[test]
fn prospective_delta_meta_validation_rejects_deterministic_sampling_atomically() {
    let mut registry = Registry::with_pure_builtins();
    let mut producer = declaration("sampled_delta", EffectClass::Pure);
    producer.delta_schema = Some(node(json!({
        "type": "str",
        "verify": [{
            "ext": "regex",
            "config": { "pattern": "x" },
            "sampling": { "samples": 1 }
        }]
    })));
    let error = registry
        .register(vec![producer], Arc::new(PassHost))
        .expect_err("delta schemas are checked against the prospective registry");
    assert!(error.contains("INVALID_SAMPLING"), "{error}");
    assert!(registry.decl("sampled_delta").is_none());
}
