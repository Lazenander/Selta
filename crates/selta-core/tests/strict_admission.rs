use selta_core::{
    validate_config_structure_with_env_holes, AdmittedNode, MetaIssue, MetaIssueCode, Node,
    SELTA_META_VALIDATOR_REVISION, SELTA_SCHEMA_LANGUAGE_REVISION,
};
use serde_json::{json, Value};

fn admit(value: Value) -> Result<AdmittedNode, Vec<MetaIssue>> {
    AdmittedNode::admit_source(&serde_json::to_vec(&value).unwrap())
}

fn issues(value: Value) -> Vec<MetaIssue> {
    admit(value).expect_err("fixture must be rejected")
}

fn has(issues: &[MetaIssue], code: MetaIssueCode, pointer: &str) -> bool {
    issues
        .iter()
        .any(|issue| issue.code == code && issue.pointer == pointer)
}

#[test]
fn admits_the_complete_closed_grammar_and_keeps_the_wrapper_opaque() {
    let admitted = admit(json!({
        "type": "object",
        "open": false,
        "description": "complete grammar fixture",
        "fields": {
            "name": {
                "type": "str",
                "required": true,
                "verify": [{ "ext": "regex", "config": { "pattern": "^[a-z]+$" } }]
            },
            "scores": {
                "type": "array",
                "required": false,
                "len": { "min": 1, "max": 3 },
                "item": {
                    "type": "union",
                    "variants": [{ "type": "int" }, { "type": "float" }]
                }
            }
        },
        "verify": [{
            "any_of": [
                { "ext": "judge", "sampling": {
                    "samples": 3,
                    "vote": { "at_least": 2 },
                    "depth": 1,
                    "min_valid": 2
                }},
                { "not": { "ext": "non_empty" }, "message": "may be empty" }
            ]
        }]
    }))
    .expect("closed fixture admits");

    assert_eq!(admitted.as_node().ty.name(), "object");
    assert_eq!(admitted.into_node().verify.len(), 1);
    assert_eq!(SELTA_SCHEMA_LANGUAGE_REVISION, "selta.schema-language/1");
    assert_eq!(SELTA_META_VALIDATOR_REVISION, "selta.meta-validator/1");
}

#[test]
fn duplicate_keys_reject_before_a_json_map_can_overwrite_them() {
    for (source, pointer) in [
        (r#"{"type":"str","type":"int"}"#, "/type"),
        (
            r#"{"type":"object","fields":{"x":{"type":"str","description":"a","description":"b"}}}"#,
            "/fields/x/description",
        ),
        (
            r#"{"type":"object","fields":{"a/b~c":{"type":"str","verify":[],"\u0076erify":[]}}}"#,
            "/fields/a~1b~0c/verify",
        ),
    ] {
        let errors = AdmittedNode::admit_source(source.as_bytes()).unwrap_err();
        assert!(
            has(&errors, MetaIssueCode::DuplicateObjectKey, pointer),
            "{errors:?}"
        );
    }
}

#[test]
fn malformed_or_trailing_json_has_one_root_addressed_issue() {
    for source in [br#"{"type":"str",}"#.as_slice(), br#"[] true"#.as_slice()] {
        let errors = AdmittedNode::admit_source(source).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, MetaIssueCode::InvalidJson);
        assert_eq!(errors[0].pointer, "");
    }
}

#[test]
fn node_keys_are_closed_per_type_and_required_is_field_only() {
    let errors = issues(json!({
        "type": "str",
        "required": true,
        "open": false,
        "fields": {},
        "descripton": "typo"
    }));
    assert!(has(
        &errors,
        MetaIssueCode::RequiredOutsideField,
        "/required"
    ));
    assert!(has(&errors, MetaIssueCode::UnexpectedProperty, "/open"));
    assert!(has(&errors, MetaIssueCode::UnexpectedProperty, "/fields"));
    assert!(has(
        &errors,
        MetaIssueCode::UnexpectedProperty,
        "/descripton"
    ));

    admit(json!({
        "type": "object",
        "fields": { "optional": { "type": "str", "required": false } }
    }))
    .expect("required is legal at a field node");
}

#[test]
fn every_type_specific_required_field_is_checked_at_its_pointer() {
    let array = issues(json!({ "type": "array" }));
    assert!(has(&array, MetaIssueCode::MissingProperty, "/item"));

    let union = issues(json!({ "type": "union" }));
    assert!(has(&union, MetaIssueCode::MissingProperty, "/variants"));

    let missing_type = issues(json!({ "description": "missing" }));
    assert!(has(&missing_type, MetaIssueCode::MissingProperty, "/type"));

    let unknown = issues(json!({ "type": "decimal" }));
    assert!(has(&unknown, MetaIssueCode::UnknownNodeType, "/type"));
}

#[test]
fn verifier_discriminants_are_exact_and_each_variant_is_closed() {
    for spec in [
        json!({}),
        json!({ "ext": "regex", "all_of": [] }),
        json!({ "ext": "regex", "message": "not a leaf field" }),
    ] {
        let errors = issues(json!({ "type": "str", "verify": [spec] }));
        if errors
            .iter()
            .any(|issue| issue.code == MetaIssueCode::VerifierDiscriminant)
        {
            assert!(has(
                &errors,
                MetaIssueCode::VerifierDiscriminant,
                "/verify/0"
            ));
        } else {
            assert!(has(
                &errors,
                MetaIssueCode::UnexpectedProperty,
                "/verify/0/message"
            ));
        }
    }
}

#[test]
fn unions_combinators_messages_and_array_bounds_are_nonempty_and_ordered() {
    let errors = issues(json!({
        "type": "union",
        "variants": [],
        "verify": [
            { "all_of": [] },
            { "any_of": [] },
            { "not": { "ext": "x" }, "message": "  " }
        ]
    }));
    assert!(has(&errors, MetaIssueCode::EmptyCollection, "/variants"));
    assert!(has(
        &errors,
        MetaIssueCode::EmptyCollection,
        "/verify/0/all_of"
    ));
    assert!(has(
        &errors,
        MetaIssueCode::EmptyCollection,
        "/verify/1/any_of"
    ));
    assert!(has(
        &errors,
        MetaIssueCode::EmptyCollection,
        "/verify/2/message"
    ));

    for (bounds, code) in [
        (json!({}), MetaIssueCode::EmptyCollection),
        (json!({ "min": 2, "max": 1 }), MetaIssueCode::InvalidBounds),
        (json!({ "min": -1 }), MetaIssueCode::TypeMismatch),
    ] {
        let errors = issues(json!({ "type": "array", "item": { "type": "str" }, "len": bounds }));
        assert!(errors.iter().any(|issue| issue.code == code), "{errors:?}");
    }
}

#[test]
fn sampling_and_vote_shapes_are_closed_and_fail_invalid_empty_policies() {
    let fixtures = [
        (
            json!({ "samples": 0 }),
            MetaIssueCode::InvalidSampling,
            "/verify/0/sampling/samples",
        ),
        (
            json!({ "min_valid": 0 }),
            MetaIssueCode::InvalidSampling,
            "/verify/0/sampling/min_valid",
        ),
        (
            json!({ "samples": 2, "vote": { "at_least": 3 } }),
            MetaIssueCode::InvalidSampling,
            "/verify/0/sampling/vote/at_least",
        ),
        (
            json!({ "vote": { "ratio": 0.0 } }),
            MetaIssueCode::InvalidSampling,
            "/verify/0/sampling/vote/ratio",
        ),
        (
            json!({ "vote": { "ratio": 0.5, "at_least": 1 } }),
            MetaIssueCode::VoteDiscriminant,
            "/verify/0/sampling/vote",
        ),
        (
            json!({ "vote": "plurality" }),
            MetaIssueCode::VoteDiscriminant,
            "/verify/0/sampling/vote",
        ),
        (
            json!({ "samples": 1, "typo": true }),
            MetaIssueCode::UnexpectedProperty,
            "/verify/0/sampling/typo",
        ),
    ];
    for (sampling, code, pointer) in fixtures {
        let errors = issues(json!({
            "type": "str",
            "verify": [{ "ext": "judge", "sampling": sampling }]
        }));
        assert!(has(&errors, code, pointer), "{errors:?}");
    }
}

#[test]
fn env_holes_are_exact_nonempty_dot_paths_at_every_depth() {
    admit(json!({
        "type": "str",
        "verify": [{
            "ext": "judge",
            "config": {
                "literal": 1,
                "nested": [{ "$env": "request.expected.value" }]
            }
        }]
    }))
    .expect("exact nested hole admits");

    for (config, pointer) in [
        (json!({ "$env": "" }), "/verify/0/config/$env"),
        (json!({ "$env": "a..b" }), "/verify/0/config/$env"),
        (json!({ "$env": 7 }), "/verify/0/config/$env"),
        (
            json!({ "$env": "a.b", "literal": true }),
            "/verify/0/config",
        ),
        (
            json!({ "nested": { "$env": "a. b" } }),
            "/verify/0/config/nested/$env",
        ),
    ] {
        let errors = issues(json!({
            "type": "str",
            "verify": [{ "ext": "judge", "config": config }]
        }));
        assert!(
            has(&errors, MetaIssueCode::InvalidEnvRef, pointer),
            "{errors:?}"
        );
    }
}

#[test]
fn config_structure_checks_literal_siblings_without_rejecting_valid_holes() {
    let config_schema = Node::from_value(json!({
        "type": "object",
        "open": false,
        "fields": {
            "pattern": { "type": "str" },
            "limit": { "type": "int", "required": false },
            "nested": {
                "type": "object",
                "fields": { "enabled": { "type": "bool" } },
                "required": false
            }
        }
    }))
    .unwrap();
    let validator = |extension: &str, config: &Value, pointer: &str| {
        if extension == "regex" {
            validate_config_structure_with_env_holes(&config_schema, config, pointer)
        } else {
            Vec::new()
        }
    };
    let source = serde_json::to_vec(&json!({
        "type": "str",
        "verify": [{
            "ext": "regex",
            "config": {
                "pattern": { "$env": "request.pattern" },
                "limit": "not-an-int",
                "nested": { "enabled": "not-a-bool" },
                "extra": true
            }
        }]
    }))
    .unwrap();
    let errors = AdmittedNode::admit_source_with_config_validator(&source, &validator).unwrap_err();
    assert!(!errors
        .iter()
        .any(|issue| issue.pointer == "/verify/0/config/pattern"));
    for pointer in [
        "/verify/0/config/limit",
        "/verify/0/config/nested/enabled",
        "/verify/0/config/extra",
    ] {
        assert!(
            has(&errors, MetaIssueCode::ConfigStructure, pointer),
            "{errors:?}"
        );
    }
}

#[test]
fn core_and_config_callbacks_do_not_duplicate_identical_env_issues() {
    let config_schema = Node::from_value(json!({ "type": "str" })).unwrap();
    let validator = |_extension: &str, config: &Value, pointer: &str| {
        validate_config_structure_with_env_holes(&config_schema, config, pointer)
    };
    let source = br#"{
        "type":"str",
        "verify":[{"ext":"x","config":{"$env":""}}]
    }"#;
    let errors = AdmittedNode::admit_source_with_config_validator(source, &validator).unwrap_err();
    assert_eq!(
        errors
            .iter()
            .filter(|issue| {
                issue.code == MetaIssueCode::InvalidEnvRef
                    && issue.pointer == "/verify/0/config/$env"
            })
            .count(),
        1,
        "{errors:?}"
    );
}

#[test]
fn issue_codes_and_json_pointers_are_stable_protocol_values() {
    let issue = MetaIssue::new(
        MetaIssueCode::InvalidDeclarationSchema,
        "/a~1b/~0key",
        "diagnostic",
    );
    assert_eq!(
        serde_json::to_value(issue).unwrap(),
        json!({
            "code": "INVALID_DECLARATION_SCHEMA",
            "pointer": "/a~1b/~0key",
            "detail": "diagnostic"
        })
    );
    assert_eq!(
        MetaIssueCode::UnknownExtension.as_str(),
        "UNKNOWN_EXTENSION"
    );
    assert_eq!(MetaIssueCode::EffectMismatch.as_str(), "EFFECT_MISMATCH");
    assert_eq!(
        MetaIssueCode::InputDomainMismatch.as_str(),
        "INPUT_DOMAIN_MISMATCH"
    );
    assert_eq!(MetaIssueCode::ConfigSemantics.as_str(), "CONFIG_SEMANTICS");
}

#[test]
fn compatibility_parser_remains_additive_and_carries_no_admission_claim() {
    let parsed = Node::from_value(json!({
        "type": "str",
        "legacy_unknown_key": true
    }))
    .expect("v0.1 compatibility remains permissive");
    assert_eq!(parsed.ty.name(), "str");

    let errors = issues(json!({ "type": "str", "legacy_unknown_key": true }));
    assert!(has(
        &errors,
        MetaIssueCode::UnexpectedProperty,
        "/legacy_unknown_key"
    ));
}
