//! End-to-end mechanics tests over a real in-process `ExtensionHost` boundary.
//! The host is deliberately deterministic test instrumentation; these tests are
//! evidence for engine mechanics, not for the semantic accuracy of an LLM judge.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use selta_core::{
    verify, AdmissionPolicy, AdmissionProfile, AssessmentEnvelope, CheckResult, Determinism,
    EffectClass, Envelope, EvidenceState, ExtensionDecl, ExtensionHost, HostCall, Input,
    InputDomain, InputKind, Needs, NoCache, Node, Options, Registry, Report, Runtime, Verdict,
    WireDelta,
};
use serde_json::{json, Value};

const STATES: [EvidenceState; 4] = [
    EvidenceState::Neither,
    EvidenceState::SupportOnly,
    EvidenceState::RefuteOnly,
    EvidenceState::Both,
];

#[derive(Default)]
struct MechanicsHost {
    verify_calls: AtomicUsize,
    assess_calls: AtomicUsize,
    sequence: AtomicUsize,
}

#[async_trait]
impl ExtensionHost for MechanicsHost {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String> {
        self.verify_calls.fetch_add(1, Ordering::Relaxed);
        match configured_state(call.config)? {
            "legacy_fail" => Ok(Envelope::fail(WireDelta::message("legacy failure"))),
            _ => Ok(Envelope::pass()),
        }
    }

    async fn assess(&self, call: HostCall<'_>) -> Result<AssessmentEnvelope, String> {
        self.assess_calls.fetch_add(1, Ordering::Relaxed);
        match configured_state(call.config)? {
            "neither" => Ok(AssessmentEnvelope::neither()),
            "support" => Ok(AssessmentEnvelope::support()),
            "refute" => Ok(AssessmentEnvelope::refute(WireDelta::message(
                "native refutation",
            ))),
            "both" => Ok(AssessmentEnvelope::both(WireDelta::message(
                "native conflict",
            ))),
            "unavailable" => Err("native assessor unavailable".to_string()),
            "repeat_support" => Ok(AssessmentEnvelope::support()),
            "alternate" => {
                let index = self.sequence.fetch_add(1, Ordering::Relaxed);
                if index.is_multiple_of(2) {
                    Ok(AssessmentEnvelope::support())
                } else {
                    Ok(AssessmentEnvelope::refute(WireDelta::message(
                        "alternating refutation",
                    )))
                }
            }
            "partial_refute" => {
                let index = self.sequence.fetch_add(1, Ordering::Relaxed);
                if index == 0 {
                    Ok(AssessmentEnvelope::refute(WireDelta::message(
                        "uncommitted refutation",
                    )))
                } else {
                    Err("partial assessor unavailable".to_string())
                }
            }
            "partial_support" => {
                let index = self.sequence.fetch_add(1, Ordering::Relaxed);
                if index == 0 {
                    Ok(AssessmentEnvelope::support())
                } else {
                    Err("partial assessor unavailable".to_string())
                }
            }
            other => Err(format!("unknown mechanics state {other:?}")),
        }
    }
}

fn configured_state(config: &Value) -> Result<&str, String> {
    config
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| "config.state must be a string".to_string())
}

struct Harness {
    registry: Registry,
    host: Arc<MechanicsHost>,
    cache: NoCache,
}

impl Harness {
    fn new() -> Self {
        let host = Arc::new(MechanicsHost::default());
        let mut registry = Registry::with_pure_builtins();
        registry
            .register(
                vec![
                    declaration("judge_det", Determinism::Deterministic),
                    declaration("judge_sample", Determinism::Nondeterministic),
                ],
                host.clone(),
            )
            .unwrap();
        Self {
            registry,
            host,
            cache: NoCache,
        }
    }

    fn schema(&self, spec: Value) -> Node {
        let source = serde_json::to_vec(&json!({
            "type": "str",
            "verify": [spec]
        }))
        .unwrap();
        self.registry
            .admit_source_at(
                &source,
                &AdmissionPolicy::pure_only(),
                AdmissionProfile::Selta2,
            )
            .unwrap()
            .into_node()
    }

    async fn run(&self, spec: Value) -> Report {
        self.run_with_options(spec, &Options::default()).await
    }

    async fn run_with_options(&self, spec: Value, options: &Options) -> Report {
        let schema = self.schema(spec);
        verify(
            &schema,
            Input::Value(json!("claim")),
            &json!({}),
            options,
            &Runtime::new(&self.registry, &self.cache),
        )
        .await
    }
}

fn declaration(name: &str, determinism: Determinism) -> ExtensionDecl {
    ExtensionDecl {
        name: name.to_string(),
        semantic_revision: format!("selta.test.{name}.presence-v1"),
        cacheable: false,
        determinism,
        effect_class: EffectClass::Pure,
        accepted_input: InputDomain::new([InputKind::Str]).unwrap(),
        config_schema: None,
        config_preflight: None,
        needs: Needs::default(),
        settings_schema: None,
        delta_schema: None,
    }
}

fn cautious_leaf(extension: &str, state: &str) -> Value {
    json!({
        "ext": extension,
        "config": { "state": state },
        "evidence": "cautious"
    })
}

fn state_name(state: EvidenceState) -> &'static str {
    match state {
        EvidenceState::Neither => "neither",
        EvidenceState::SupportOnly => "support",
        EvidenceState::RefuteOnly => "refute",
        EvidenceState::Both => "both",
    }
}

fn semantic_check(report: &Report) -> &CheckResult {
    report
        .root
        .checks
        .iter()
        .find(|check| check.evidence.is_some())
        .expect("evidence-mode check must retain its summary")
}

fn assert_projected(report: &Report, expected: EvidenceState) {
    let check = semantic_check(report);
    let summary = check.evidence.as_ref().unwrap();
    assert_eq!(summary.state, Some(expected));
    assert_eq!(check.verdict, expected.project());
    assert_eq!(report.verdict, expected.project());
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(check.error, None);
    match expected {
        EvidenceState::RefuteOnly => {
            assert!(!summary.refutations.is_empty());
            assert!(!check.deltas.is_empty());
            assert!(!report.deltas.is_empty());
        }
        EvidenceState::Both => {
            assert!(!summary.refutations.is_empty());
            assert!(check.deltas.is_empty());
            assert!(report.deltas.is_empty());
        }
        EvidenceState::Neither | EvidenceState::SupportOnly => {
            assert!(summary.refutations.is_empty());
            assert!(check.deltas.is_empty());
            assert!(report.deltas.is_empty());
        }
    }
}

#[tokio::test]
async fn native_deterministic_assessor_expresses_all_four_states_in_one_call() {
    let harness = Harness::new();
    for state in STATES {
        let report = harness
            .run(cautious_leaf("judge_det", state_name(state)))
            .await;
        assert_projected(&report, state);
        let summary = semantic_check(&report).evidence.as_ref().unwrap();
        assert_eq!(
            summary.neither + summary.support_only + summary.refute_only + summary.both,
            1
        );
        assert_eq!(summary.unavailable, 0);
    }
    assert_eq!(harness.host.assess_calls.load(Ordering::Relaxed), 4);
    assert_eq!(harness.host.verify_calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn semantic_neither_and_operational_unavailability_remain_distinct() {
    let harness = Harness::new();
    let neither = harness.run(cautious_leaf("judge_det", "neither")).await;
    let neither_summary = semantic_check(&neither).evidence.as_ref().unwrap();
    assert_eq!(neither_summary.state, Some(EvidenceState::Neither));
    assert_eq!(neither_summary.neither, 1);
    assert_eq!(neither_summary.unavailable, 0);
    assert!(neither.errors.is_empty());

    let unavailable = harness.run(cautious_leaf("judge_det", "unavailable")).await;
    let unavailable_check = semantic_check(&unavailable);
    let unavailable_summary = unavailable_check.evidence.as_ref().unwrap();
    assert_eq!(unavailable_summary.state, None);
    assert_eq!(unavailable_summary.neither, 0);
    assert_eq!(unavailable_summary.unavailable, 1);
    assert_eq!(unavailable_check.verdict, Verdict::Inconclusive);
    assert!(unavailable_check.error.is_some());
    assert_eq!(unavailable.errors.len(), 1);
    assert!(unavailable.deltas.is_empty());
}

#[tokio::test]
async fn repeated_same_polarity_changes_counts_but_not_state() {
    let harness = Harness::new();
    let report = harness
        .run(json!({
            "ext": "judge_sample",
            "config": { "state": "repeat_support" },
            "evidence": "cautious",
            "sampling": { "samples": 3, "min_valid": 3 }
        }))
        .await;
    assert_projected(&report, EvidenceState::SupportOnly);
    let summary = semantic_check(&report).evidence.as_ref().unwrap();
    assert_eq!(summary.support_only, 3);
    assert_eq!(summary.state, Some(EvidenceState::SupportOnly));
    assert_eq!(harness.host.assess_calls.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn sampled_opposing_polarities_preserve_both_without_failure_deltas() {
    let harness = Harness::new();
    let report = harness
        .run(json!({
            "ext": "judge_sample",
            "config": { "state": "alternate" },
            "evidence": "cautious",
            "sampling": { "samples": 4, "min_valid": 4 }
        }))
        .await;
    assert_projected(&report, EvidenceState::Both);
    let summary = semantic_check(&report).evidence.as_ref().unwrap();
    assert_eq!(summary.support_only, 2);
    assert_eq!(summary.refute_only, 2);
    assert_eq!(summary.both, 0);
    assert_eq!(harness.host.assess_calls.load(Ordering::Relaxed), 4);
}

#[tokio::test]
async fn unscheduled_assessments_are_explicitly_unavailable_when_budget_is_short() {
    for (max_samples, completed, unavailable) in [(1, 1, 4), (0, 0, 5)] {
        let harness = Harness::new();
        let report = harness
            .run_with_options(
                json!({
                    "ext": "judge_sample",
                    "config": { "state": "repeat_support" },
                    "evidence": "cautious",
                    "sampling": { "samples": 5, "min_valid": 3 }
                }),
                &Options {
                    max_samples,
                    ..Options::default()
                },
            )
            .await;
        let check = semantic_check(&report);
        let summary = check.evidence.as_ref().unwrap();
        assert_eq!(summary.support_only, completed);
        assert_eq!(summary.unavailable, unavailable);
        assert_eq!(
            summary.state,
            (completed > 0).then_some(EvidenceState::SupportOnly)
        );
        assert_eq!(check.verdict, Verdict::Inconclusive);
        assert!(check
            .error
            .as_deref()
            .is_some_and(|error| error.contains("quorum")));
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].error.contains("sample budget exhausted"));
        assert_eq!(report.errors[0].samples_lost, unavailable);
    }
}

#[tokio::test]
async fn incomplete_child_retains_observations_without_lending_them_to_composition() {
    let harness = Harness::new();
    let report = harness
        .run(json!({ "all_of": [
            { "all_of": [
                {
                    "ext": "judge_sample",
                    "config": { "state": "partial_refute" },
                    "evidence": "cautious",
                    "sampling": { "samples": 2, "min_valid": 2 }
                },
                cautious_leaf("judge_det", "refute")
            ] },
            cautious_leaf("judge_det", "support")
        ] }))
        .await;

    let check = semantic_check(&report);
    let summary = check.evidence.as_ref().unwrap();
    assert_eq!(summary.state, Some(EvidenceState::RefuteOnly));
    assert_eq!(summary.refute_only, 2);
    assert_eq!(summary.support_only, 1);
    assert_eq!(summary.unavailable, 3);
    assert_eq!(check.verdict, Verdict::Fail);
    assert_eq!(
        check.error, None,
        "a decisive parent does not claim that its own verdict is erroneous"
    );
    assert_eq!(summary.refutations.len(), 1);
    assert_eq!(summary.refutations[0].message, "native refutation");
    assert_eq!(check.deltas, summary.refutations);
    assert_eq!(report.deltas, summary.refutations);
    assert_eq!(report.errors.len(), 1);
}

#[tokio::test]
async fn completed_any_of_support_remains_usable_above_an_incomplete_sibling() {
    let harness = Harness::new();
    let report = harness
        .run(json!({ "any_of": [
            { "any_of": [
                {
                    "ext": "judge_sample",
                    "config": { "state": "partial_support" },
                    "evidence": "cautious",
                    "sampling": { "samples": 2, "min_valid": 2 }
                },
                cautious_leaf("judge_det", "support")
            ] },
            cautious_leaf("judge_det", "refute")
        ] }))
        .await;

    let check = semantic_check(&report);
    let summary = check.evidence.as_ref().unwrap();
    assert_eq!(summary.state, Some(EvidenceState::SupportOnly));
    assert_eq!(summary.support_only, 2);
    assert_eq!(summary.refute_only, 1);
    assert_eq!(summary.unavailable, 3);
    assert_eq!(check.verdict, Verdict::Pass);
    assert_eq!(check.error, None);
    assert!(summary.refutations.is_empty());
    assert!(check.deltas.is_empty());
    assert!(report.deltas.is_empty());
    assert_eq!(report.errors.len(), 1);
}

#[tokio::test]
async fn cautious_combinators_follow_the_four_valued_equations() {
    let harness = Harness::new();
    for left in STATES {
        for right in STATES {
            let all_of = harness
                .run(json!({ "all_of": [
                    cautious_leaf("judge_det", state_name(left)),
                    cautious_leaf("judge_det", state_name(right))
                ] }))
                .await;
            assert_eq!(
                semantic_check(&all_of).verdict,
                left.conjoin(right).project(),
                "all_of({left:?}, {right:?})"
            );
            assert_projected(&all_of, left.conjoin(right));

            let any_of = harness
                .run(json!({ "any_of": [
                    cautious_leaf("judge_det", state_name(left)),
                    cautious_leaf("judge_det", state_name(right))
                ] }))
                .await;
            assert_eq!(
                semantic_check(&any_of).verdict,
                left.disjoin(right).project(),
                "any_of({left:?}, {right:?})"
            );
            assert_projected(&any_of, left.disjoin(right));
        }

        let negated = harness
            .run(json!({
                "not": cautious_leaf("judge_det", state_name(left)),
                "message": "the inner claim was supported"
            }))
            .await;
        assert_eq!(
            semantic_check(&negated).verdict,
            left.negate().project(),
            "not({left:?})"
        );
        assert_projected(&negated, left.negate());
    }
}

#[tokio::test]
async fn legacy_leaf_keeps_the_binary_lane_and_omits_evidence() {
    let harness = Harness::new();
    let report = harness
        .run(json!({
            "ext": "judge_det",
            "config": { "state": "legacy_pass" }
        }))
        .await;
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.errors.is_empty());
    assert!(report
        .root
        .checks
        .iter()
        .all(|check| check.evidence.is_none()));
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("\"evidence\""));
    assert_eq!(harness.host.verify_calls.load(Ordering::Relaxed), 1);
    assert_eq!(harness.host.assess_calls.load(Ordering::Relaxed), 0);
}
