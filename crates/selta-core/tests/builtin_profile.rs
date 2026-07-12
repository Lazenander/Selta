//! Builtin declarations own their effect, input, and config-admission contract.

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use selta_core::{CmdTemplate, CmdTemplates, EffectClass, InputKind, Options, Registry, Verdict};
use serde_json::{json, Value};

use common::schema;

#[test]
fn pure_registry_excludes_cmd_and_provider_gates_the_full_profile() {
    let pure = Registry::with_pure_builtins();
    assert_eq!(
        pure.names(),
        ["len", "non_empty", "one_of", "range", "regex"]
    );
    assert!(pure
        .decls()
        .iter()
        .all(|declaration| declaration.effect_class == EffectClass::Pure));
    assert!(Registry::with_builtins(None).decl("cmd").is_none());

    let provider: Arc<dyn CmdTemplates> = Arc::new(HashMap::<String, CmdTemplate>::new());
    let full = Registry::with_builtins(Some(provider));
    let cmd = full.decl("cmd").expect("provider enables cmd declaration");
    assert_eq!(cmd.effect_class, EffectClass::ProcessIo);
    assert!(!cmd.cacheable);
}

#[test]
fn builtin_declarations_publish_exact_input_domains() {
    let registry = Registry::with_pure_builtins();
    assert_eq!(
        registry.decl("regex").unwrap().accepted_input.kinds(),
        &[InputKind::Str]
    );
    assert_eq!(
        registry.decl("range").unwrap().accepted_input.kinds(),
        &[InputKind::Int, InputKind::Float]
    );
    assert_eq!(
        registry.decl("len").unwrap().accepted_input.kinds(),
        &[InputKind::Str, InputKind::Array]
    );
    assert_eq!(
        registry.decl("non_empty").unwrap().accepted_input.kinds(),
        &[InputKind::Str, InputKind::Object, InputKind::Array]
    );
    assert_eq!(
        registry.decl("one_of").unwrap().accepted_input.kinds(),
        &InputKind::ALL
    );
}

#[test]
fn meta_rejects_builtin_node_input_domain_mismatches() {
    let registry = Registry::with_pure_builtins();
    for raw in [
        json!({ "type": "int", "verify": [{ "ext": "regex", "config": { "pattern": "x" } }] }),
        json!({ "type": "str", "verify": [{ "ext": "range", "config": { "min": 0 } }] }),
        json!({ "type": "bool", "verify": [{ "ext": "len", "config": { "min": 1 } }] }),
        json!({ "type": "bool", "verify": [{ "ext": "non_empty" }] }),
        json!({
            "type": "union",
            "variants": [{ "type": "str" }, { "type": "int" }],
            "verify": [{ "ext": "regex", "config": { "pattern": "x" } }]
        }),
    ] {
        let node = schema(raw);
        let errors = selta_core::meta::validate(&node, &registry);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("does not accept node type")),
            "{errors:?}"
        );
    }
}

#[test]
fn literal_builtin_configs_receive_semantic_preflight() {
    let registry = Registry::with_pure_builtins();
    let invalid = [
        json!({ "type": "str", "verify": [{ "ext": "one_of", "config": { "values": [] } }] }),
        json!({ "type": "float", "verify": [{ "ext": "range", "config": {} }] }),
        json!({ "type": "float", "verify": [{ "ext": "range", "config": { "min": 2, "max": 1 } }] }),
        json!({ "type": "str", "verify": [{ "ext": "len", "config": {} }] }),
        json!({ "type": "str", "verify": [{ "ext": "len", "config": { "min": -1 } }] }),
        json!({ "type": "str", "verify": [{ "ext": "len", "config": { "min": 2, "max": 1 } }] }),
        json!({ "type": "str", "verify": [{ "ext": "regex", "config": { "pattern": "[" } }] }),
    ];

    for raw in invalid {
        let errors = selta_core::meta::validate(&schema(raw), &registry);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("failed semantic preflight")),
            "{errors:?}"
        );
    }
}

#[tokio::test]
async fn resolved_dynamic_config_reuses_the_same_preflight() {
    let registry = Registry::with_pure_builtins();
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "regex",
            "config": { "pattern": { "$env": "pattern" } }
        }]
    }));
    assert!(
        selta_core::meta::validate(&node, &registry).is_empty(),
        "the literal schema contains a well-shaped unresolved hole"
    );

    let report = common::run(
        &node,
        Value::String("value".to_string()),
        json!({ "pattern": "[" }),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert!(report.deltas.is_empty());
    assert!(report.errors[0].error.contains("semantic preflight"));
}
