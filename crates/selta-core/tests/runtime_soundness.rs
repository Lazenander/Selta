mod common;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use selta_core::{
    CmdInput, CmdTemplate, Determinism, EffectClass, Envelope, ExtensionDecl, ExtensionHost,
    HostCall, Input, InputDomain, Mode, Needs, NoCache, Options, Registry, Verdict, WireUsage,
    MAX_SAMPLE_IN_FLIGHT_PER_JOB,
};
use serde_json::json;

use common::{err, fail, registry_with, schema, ScriptedHost};

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[derive(Default)]
struct ConcurrencyProbeHost {
    active: AtomicU32,
    peak: AtomicU32,
    calls: AtomicU32,
    errors_remaining: AtomicU32,
}

impl ConcurrencyProbeHost {
    fn observe_peak(&self, active: u32) {
        let mut peak = self.peak.load(Ordering::SeqCst);
        while active > peak {
            match self
                .peak
                .compare_exchange_weak(peak, active, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => break,
                Err(observed) => peak = observed,
            }
        }
    }
}

struct ActiveCall<'a>(&'a AtomicU32);

impl Drop for ActiveCall<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl ExtensionHost for ConcurrencyProbeHost {
    async fn verify(&self, _call: HostCall<'_>) -> Result<Envelope, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        let _active = ActiveCall(&self.active);
        self.observe_peak(active);
        // Every future in the current join set reaches this yield before any
        // completes, making peak concurrency an exact proxy for batch size.
        tokio::task::yield_now().await;
        if self
            .errors_remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            Err("retryable probe error".to_string())
        } else {
            Ok(Envelope::pass())
        }
    }
}

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
async fn large_sampling_requests_are_materialized_in_bounded_windows() {
    let requested = MAX_SAMPLE_IN_FLIGHT_PER_JOB * 4 + 7;
    let retry_errors = MAX_SAMPLE_IN_FLIGHT_PER_JOB + 7;
    let host = Arc::new(ConcurrencyProbeHost {
        errors_remaining: AtomicU32::new(retry_errors),
        ..ConcurrencyProbeHost::default()
    });
    let mut registry = Registry::with_builtins(None);
    registry
        .register(
            vec![ExtensionDecl {
                name: "windowed_judge".to_string(),
                semantic_revision: "selta.test.windowed-judge.v1".to_string(),
                cacheable: false,
                determinism: Determinism::Nondeterministic,
                effect_class: EffectClass::Pure,
                accepted_input: InputDomain::any(),
                config_schema: None,
                config_preflight: None,
                needs: Needs::default(),
                settings_schema: None,
                delta_schema: None,
            }],
            host.clone(),
        )
        .expect("probe registers");
    let node = schema(json!({
        "type": "str",
        "verify": [{
            "ext": "windowed_judge",
            "sampling": {
                "samples": requested,
                "vote": "unanimous",
                "depth": 1,
                "min_valid": requested
            }
        }]
    }));
    let report = common::run(
        &node,
        json!("value"),
        json!({}),
        &Options {
            max_samples: requested + retry_errors,
            ..Options::default()
        },
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(
        host.calls.load(Ordering::SeqCst),
        requested + retry_errors,
        "failed executions are resampled across windows"
    );
    assert_eq!(
        host.peak.load(Ordering::SeqCst),
        MAX_SAMPLE_IN_FLIGHT_PER_JOB,
        "no join set may materialize more than the public in-flight ceiling"
    );
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

#[tokio::test]
async fn a_timed_out_cmd_child_cannot_continue_after_the_report_returns() {
    if !node_available() {
        eprintln!("skipping timeout containment test: node not found");
        return;
    }
    let scratch = tempfile::tempdir().unwrap();
    let marker = scratch.path().join("late-side-effect");
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/late_side_effect.mjs"
    );
    let mut templates = HashMap::new();
    templates.insert(
        "timeout_probe".to_string(),
        CmdTemplate {
            run: vec!["node".to_string(), fixture.to_string()],
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
                "args": [marker.to_str().expect("temporary marker path is Unicode")]
            }
        }]
    }));
    let report = common::run(
        &node,
        json!("x".repeat(2 * 1024 * 1024)),
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

#[tokio::test]
async fn a_cmd_child_can_read_its_closed_temporary_input_file() {
    if !node_available() {
        eprintln!("skipping temporary-file sharing test: node not found");
        return;
    }
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/read_cmd_input.mjs"
    );
    let mut templates = HashMap::new();
    templates.insert(
        "file_probe".to_string(),
        CmdTemplate {
            run: vec![
                "node".to_string(),
                fixture.to_string(),
                "{file}".to_string(),
            ],
            input: CmdInput::File,
            timeout_ms: 1_000,
        },
    );
    let registry = Registry::with_builtins(Some(Arc::new(templates)));
    let node = schema(json!({
        "type": "str",
        "verify": [{ "ext": "cmd", "config": { "name": "file_probe" } }]
    }));
    let report = common::run(
        &node,
        json!("portable payload"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Pass, "{report:?}");
}
