//! The docs/01 worked example, deterministic slice — the M1 acceptance test.

mod common;

use selta_core::{DeltaKind, Options, Registry, Verdict};
use serde_json::json;

fn bugfix_schema() -> selta_core::Node {
    common::schema(json!({
        "type": "object",
        "fields": {
            "language": {
                "type": "str",
                "verify": [ { "ext": "one_of", "config": { "values": ["rust", "python"] } } ]
            },
            "code": { "type": "str" },
            "confidence": {
                "type": "float",
                "verify": [ { "ext": "range", "config": { "min": 0.0, "max": 1.0 } } ]
            },
            "tests": {
                "type": "array", "item": { "type": "str" }, "required": false,
                "verify": [ { "ext": "len", "config": { "min": 1 } } ]
            }
        }
    }))
}

#[tokio::test]
async fn attempt_one_fails_exactly_where_the_docs_say() {
    let registry = Registry::with_builtins(None);
    let report = common::run(
        &bugfix_schema(),
        json!({
            "language": "rust",
            "code": "fn fix(x: Option<i32>) -> i32 { x.unwrap() }",
            "confidence": 1.7
        }),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(report.deltas.len(), 1);
    let delta = &report.deltas[0];
    assert_eq!(delta.path, "$.confidence");
    assert_eq!(delta.kind, DeltaKind::Constraint);
    assert_eq!(delta.source, "range");
    assert_eq!(delta.expected.as_deref(), Some("0..=1"));
    assert_eq!(delta.actual.as_deref(), Some("1.7"));
    assert!(report.errors.is_empty());
}

#[tokio::test]
async fn attempt_two_passes() {
    let registry = Registry::with_builtins(None);
    let report = common::run(
        &bugfix_schema(),
        json!({
            "language": "rust",
            "code": "fn fix(x: Option<i32>) -> i32 { x.unwrap_or(0) }",
            "confidence": 0.85,
            "tests": ["assert_eq!(fix(None), 0);"]
        }),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.deltas.is_empty());
}

#[tokio::test]
async fn optional_field_present_but_empty_fails_len() {
    let registry = Registry::with_builtins(None);
    let report = common::run(
        &bugfix_schema(),
        json!({
            "language": "python",
            "code": "def fix(x): return x or 0",
            "confidence": 0.5,
            "tests": []
        }),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(report.deltas.len(), 1);
    assert_eq!(report.deltas[0].path, "$.tests");
    assert_eq!(report.deltas[0].source, "len");
}
