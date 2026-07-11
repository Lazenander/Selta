mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use selta_core::{
    CmdInput, CmdTemplate, Determinism, Envelope, Input, Mode, NoCache, Options, Registry, Verdict,
    WireUsage,
};
use serde_json::json;

use common::{err, fail, registry_with, schema, ScriptedHost};

#[test]
fn meta_validation_rejects_inverted_array_bounds() {
    let node = schema(json!({
        "type": "array",
        "len": { "min": 2, "max": 1 },
        "item": { "type": "str" }
    }));
    let errors = selta_core::meta::validate(&node, &Registry::with_builtins(None));
    assert!(errors
        .iter()
        .any(|error| error.contains("len.min must not exceed len.max")));
}

#[tokio::test]
async fn fail_fast_from_a_rejected_union_branch_cannot_skip_the_next_branch() {
    let node = schema(json!({
        "type": "union",
        "variants": [
            { "type": "str" },
            { "type": "object", "fields": { "x": { "type": "int" } } }
        ]
    }));
    let registry = Registry::with_builtins(None);
    let report = selta_core::verify(
        &node,
        Input::Value(json!({ "x": "not-an-int" })),
        &json!({}),
        &Options {
            mode: Mode::Strict,
            fail_fast: true,
            ..Options::default()
        },
        &selta_core::Runtime::new(&registry, &NoCache),
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert!(!report.deltas.is_empty());
}

#[tokio::test]
async fn a_parseable_empty_union_is_a_structural_failure_not_a_panic() {
    let node = schema(json!({ "type": "union", "variants": [] }));
    let registry = Registry::with_builtins(None);
    let report = selta_core::verify(
        &node,
        Input::Value(json!(null)),
        &json!({}),
        &Options::default(),
        &selta_core::Runtime::new(&registry, &NoCache),
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert!(report.deltas[0].message.contains("at least one variant"));
}

#[tokio::test]
async fn an_inconclusive_delta_schema_never_becomes_failure_evidence() {
    let delta_schema = schema(json!({
        "type": "object",
        "fields": { "message": { "type": "str" } },
        "verify": [{
            "ext": "judge",
            "sampling": { "samples": 1, "depth": 1, "min_valid": 1 }
        }]
    }));
    let host = ScriptedHost::script(vec![fail("outer fail"), fail("outer fail")]);
    let registry = registry_with(
        "judge",
        Determinism::Nondeterministic,
        Some(delta_schema),
        host,
    );
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "judge",
            "sampling": { "samples": 1, "depth": 1, "min_valid": 1 }
        }]
    }));
    let report = common::run(
        &node,
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert!(report.deltas.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.error.contains("delta schema was inconclusive")));
}

#[tokio::test]
async fn overflowing_host_usage_is_an_error_and_never_panics_or_wraps() {
    let with_usage = || {
        let mut envelope = Envelope::pass();
        envelope.usage = Some(WireUsage {
            input_tokens: u64::MAX,
            output_tokens: u64::MAX,
            cost_usd: f64::MAX,
        });
        Ok(envelope)
    };
    let host = ScriptedHost::script(vec![with_usage(), with_usage()]);
    let registry = registry_with("metered", Determinism::Deterministic, None, host);
    let node = schema(json!({
        "type": "str",
        "verify": [
            { "ext": "metered" },
            { "ext": "metered" }
        ]
    }));
    let report = common::run(
        &node,
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_eq!(report.usage.input_tokens, u64::MAX);
    assert!(report
        .errors
        .iter()
        .any(|error| error.error.contains("usage overflow")));
}

#[tokio::test]
async fn a_strict_non_integral_int_mismatch_suppresses_attached_verifiers() {
    let host = ScriptedHost::script(vec![err("must never run")]);
    let registry = registry_with("probe", Determinism::Deterministic, None, host.clone());
    let node = schema(json!({
        "type": "int",
        "verify": [{ "ext": "probe" }]
    }));
    let report = common::run(
        &node,
        json!(1.5),
        json!({}),
        &Options {
            mode: Mode::Strict,
            ..Options::default()
        },
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(host.calls(), 0);
    assert!(report.errors.is_empty());
}

#[tokio::test]
async fn zero_sample_and_zero_quorum_inputs_fail_closed_without_allocation() {
    let host = ScriptedHost::script(Vec::new());
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host.clone());
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "judge",
            "sampling": {
                "samples": 0,
                "vote": "unanimous",
                "depth": 1,
                "min_valid": 0
            }
        }]
    }));
    let meta_errors = selta_core::meta::validate(&node, &registry);
    assert!(meta_errors.iter().any(|error| error.contains("samples")));
    assert!(meta_errors.iter().any(|error| error.contains("min_valid")));

    let report = common::run(
        &node,
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert_eq!(host.calls(), 0);
}

#[tokio::test]
async fn at_least_zero_is_rejected_and_cannot_turn_a_failing_vote_into_pass() {
    let host = ScriptedHost::script(vec![fail("still wrong")]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host);
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "judge",
            "sampling": {
                "samples": 1,
                "vote": { "at_least": 0 },
                "depth": 1,
                "min_valid": 1
            }
        }]
    }));
    assert!(selta_core::meta::validate(&node, &registry)
        .iter()
        .any(|error| error.contains("at_least must be at least 1")));

    let report = common::run(
        &node,
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Fail);
}

#[cfg(unix)]
#[tokio::test]
async fn a_timed_out_cmd_child_cannot_continue_after_the_report_returns() {
    let scratch = tempfile::tempdir().unwrap();
    let marker = scratch.path().join("late-side-effect");
    let mut templates = HashMap::new();
    templates.insert(
        "timeout_probe".to_string(),
        CmdTemplate {
            run: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "sleep 0.2; touch \"$1\"".to_string(),
                "selta-timeout-probe".to_string(),
            ],
            input: CmdInput::Stdin,
            timeout_ms: 20,
        },
    );
    let registry = Registry::with_builtins(Some(Arc::new(templates)));
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "cmd",
            "config": {
                "name": "timeout_probe",
                "args": [marker.to_string_lossy()]
            }
        }]
    }));
    let report = common::run(
        &node,
        json!("payload"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Inconclusive);

    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(
        !marker.exists(),
        "timed-out child survived and touched marker"
    );
}
