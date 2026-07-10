//! Pins the semantics of docs/03: K3 laws, voting, quorum, depth,
//! combinators, dynamic `$env` config, intake, unions, caching.

mod common;

use serde_json::json;
use selta_core::{
    Determinism, Input, MemoryCache, Mode, NoCache, Options, Registry, Verdict,
};

use common::{err, fail, fail_with_data, pass, registry_with, schema, ScriptedHost};

// --- K3 laws (docs/08): free property tests over all verdict pairs ---

#[test]
fn k3_laws_hold() {
    use Verdict::{Fail, Inconclusive, Pass};
    let all = [Pass, Fail, Inconclusive];
    for a in all {
        for b in all {
            assert_eq!(a.and(b), b.and(a), "and commutes");
            assert_eq!(a.or(b), b.or(a), "or commutes");
            assert_eq!(a.and(b).negate(), a.negate().or(b.negate()), "De Morgan");
            for c in all {
                assert_eq!(a.and(b).and(c), a.and(b.and(c)), "and associates");
                assert_eq!(a.or(b).or(c), a.or(b.or(c)), "or associates");
            }
        }
        assert_eq!(a.negate().negate(), a, "double negation");
    }
}

// --- voting (docs/03 §Voting) ---

fn judge_schema(samples: u32, vote: serde_json::Value) -> selta_core::Node {
    schema(json!({
        "type": "str",
        "verify": [ { "ext": "judge", "config": {},
                      "sampling": { "samples": samples, "vote": vote, "depth": 1 } } ]
    }))
}

#[tokio::test]
async fn majority_passes_two_of_three() {
    let host = ScriptedHost::script(vec![pass(), pass(), fail("nope")]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host.clone());
    let report = common::run(
        &judge_schema(3, json!("majority")),
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;
    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(host.calls(), 3);
    let check = &report.root.checks[1];
    let tally = check.votes.expect("tally recorded");
    assert_eq!((tally.pass, tally.fail, tally.errors), (2, 1, 0));
}

#[tokio::test]
async fn at_least_fails_and_merges_distinct_critiques() {
    let host = ScriptedHost::script(vec![
        pass(),
        fail("unwrap() still panics on None"),
        fail("unwrap() still panics on None"),
        fail("the bug is not fixed"),
        fail("unwrap() still panics on None"),
    ]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host);
    let report = common::run(
        &judge_schema(5, json!({ "at_least": 4 })),
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(report.deltas.len(), 1, "failing votes merge into one delta");
    let delta = &report.deltas[0];
    assert!(delta.message.contains("unwrap() still panics on None"));
    assert!(delta.message.contains("the bug is not fixed"));
    assert_eq!(
        delta.message.matches("unwrap() still panics").count(),
        1,
        "identical messages deduplicate"
    );
    let tally = delta.votes.expect("tally attached to merged delta");
    assert_eq!((tally.pass, tally.fail), (1, 4));
}

#[tokio::test]
async fn quorum_failure_is_inconclusive_never_fail() {
    let host = ScriptedHost::script(vec![
        err("api down"), err("api down"), err("api down"),
        err("api down"), err("api down"), err("api down"),
    ]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host);
    let report = common::run(
        &judge_schema(3, json!("majority")),
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert!(report.deltas.is_empty(), "errors never become deltas");
    assert!(!report.errors.is_empty());
    assert!(report.errors[0].error.contains("quorum not met"));
}

#[tokio::test]
async fn malformed_delta_is_rejected_and_resampled() {
    // delta_schema requires structured data: a bare message is not a valid vote.
    let delta_schema = schema(json!({
        "type": "object",
        "fields": {
            "message": { "type": "str" },
            "data": { "type": "object", "fields": { "reason": { "type": "str" } } }
        }
    }));
    let host = ScriptedHost::script(vec![
        fail("unstructured critique"), // invalid: no data.reason → error, resampled
        pass(),
        pass(),
        pass(),
    ]);
    let registry = registry_with(
        "judge",
        Determinism::Nondeterministic,
        Some(delta_schema),
        host.clone(),
    );
    let report = common::run(
        &judge_schema(3, json!("majority")),
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(host.calls(), 4, "the malformed sample was replaced");
    let tally = report.root.checks[1].votes.expect("tally");
    assert_eq!((tally.pass, tally.errors), (3, 1));
}

#[tokio::test]
async fn structured_delta_satisfying_its_schema_counts_as_a_vote() {
    let delta_schema = schema(json!({
        "type": "object",
        "fields": {
            "message": { "type": "str" },
            "data": { "type": "object", "fields": { "reason": { "type": "str" } } }
        }
    }));
    let host = ScriptedHost::script(vec![
        fail_with_data("bad fix", json!({ "reason": "still panics" })),
        fail_with_data("bad fix", json!({ "reason": "still panics" })),
        pass(),
    ]);
    let registry = registry_with("judge", Determinism::Nondeterministic, Some(delta_schema), host);
    let report = common::run(
        &judge_schema(3, json!("majority")),
        json!("value"),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(report.deltas[0].data, Some(json!({ "reason": "still panics" })));
}

// --- depth (docs/03): step-indexed stratification ---

#[tokio::test]
async fn depth_zero_skips_nondeterministic_checks() {
    let host = ScriptedHost::script(vec![]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host.clone());
    let options = Options {
        max_depth: 0,
        ..Options::default()
    };
    let report = common::run(
        &judge_schema(3, json!("majority")),
        json!("value"),
        json!({}),
        &options,
        &registry,
    )
    .await;

    assert_eq!(host.calls(), 0, "judge never ran at depth 0");
    assert_eq!(
        report.verdict,
        Verdict::Inconclusive,
        "a skipped check was this node's only check — never a silent pass"
    );
    assert!(report
        .notices
        .iter()
        .any(|n| n.message.contains("depth exhausted")));
}

#[tokio::test]
async fn skipped_check_is_ignored_when_others_executed() {
    let host = ScriptedHost::script(vec![]);
    let registry = registry_with("judge", Determinism::Nondeterministic, None, host);
    let node = schema(json!({
        "type": "str",
        "verify": [
            { "ext": "regex", "config": { "pattern": "^v" } },
            { "ext": "judge", "config": {}, "sampling": { "samples": 1, "depth": 1 } }
        ]
    }));
    let options = Options {
        max_depth: 0,
        ..Options::default()
    };
    let report = common::run(&node, json!("value"), json!({}), &options, &registry).await;
    assert_eq!(report.verdict, Verdict::Pass);
}

// --- combinators (docs/02 §Composition, docs/03 §Verifier composition) ---

#[tokio::test]
async fn any_of_passes_on_first_pass_and_reports_best_failure() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "str",
        "verify": [ { "any_of": [
            { "ext": "regex", "config": { "pattern": "^a" } },
            { "ext": "regex", "config": { "pattern": "^b" } }
        ] } ]
    }));
    let options = Options::default();

    let passing = common::run(&node, json!("bcd"), json!({}), &options, &registry).await;
    assert_eq!(passing.verdict, Verdict::Pass, "second branch passes");

    let failing = common::run(&node, json!("zzz"), json!({}), &options, &registry).await;
    assert_eq!(failing.verdict, Verdict::Fail);
    assert_eq!(failing.deltas.len(), 1, "only the best branch's deltas");
}

#[tokio::test]
async fn not_inverts_and_uses_the_schema_authors_message() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "str",
        "verify": [ { "not": { "ext": "regex", "config": { "pattern": "TODO" } },
                      "message": "output must not contain TODO markers" } ]
    }));
    let options = Options::default();

    let clean = common::run(&node, json!("done"), json!({}), &options, &registry).await;
    assert_eq!(clean.verdict, Verdict::Pass);

    let dirty = common::run(&node, json!("TODO: fix"), json!({}), &options, &registry).await;
    assert_eq!(dirty.verdict, Verdict::Fail);
    assert_eq!(dirty.deltas[0].message, "output must not contain TODO markers");
}

// --- dynamic config: the regular language arrives with the request ---

#[tokio::test]
async fn regex_pattern_from_env_online() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "str",
        "verify": [ { "ext": "regex", "config": { "pattern": { "$env": "expected_pattern" } } } ]
    }));
    let options = Options::default();

    let ok = common::run(
        &node,
        json!("hello"),
        json!({ "expected_pattern": "^[a-z]+$" }),
        &options,
        &registry,
    )
    .await;
    assert_eq!(ok.verdict, Verdict::Pass);

    let bad = common::run(
        &node,
        json!("Hello9"),
        json!({ "expected_pattern": "^[a-z]+$" }),
        &options,
        &registry,
    )
    .await;
    assert_eq!(bad.verdict, Verdict::Fail);
    assert!(bad.deltas[0].message.contains("does not match"));
    assert_eq!(bad.deltas[0].expected.as_deref(), Some("/^[a-z]+$/"));
}

#[tokio::test]
async fn missing_env_reference_is_inconclusive_not_fail() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "str",
        "verify": [ { "ext": "regex", "config": { "pattern": { "$env": "expected_pattern" } } } ]
    }));
    let report = common::run(&node, json!("hello"), json!({}), &Options::default(), &registry).await;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert!(report.deltas.is_empty(), "the value is not wrong — the request is");
    assert!(report.errors[0].error.contains("missing env field 'expected_pattern'"));
}

// --- intake (docs/03 §1) ---

#[tokio::test]
async fn fenced_json_with_trailing_comma_repairs_with_notices() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({ "type": "object", "fields": { "a": { "type": "int" } } }));
    let report = selta_core::verify(
        &node,
        Input::Text("```json\n{ \"a\": 1, }\n```"),
        &json!({}),
        &Options::default(),
        &selta_core::Runtime::new(&registry, &NoCache),
    )
    .await;

    assert_eq!(report.verdict, Verdict::Pass);
    let messages: Vec<&str> = report.notices.iter().map(|n| n.message.as_str()).collect();
    assert!(messages.iter().any(|m| m.contains("code fence")));
    assert!(messages.iter().any(|m| m.contains("trailing comma")));
}

#[tokio::test]
async fn strict_mode_rejects_fences_and_coercions() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({ "type": "float" }));
    let options = Options {
        mode: Mode::Strict,
        ..Options::default()
    };

    let fenced = selta_core::verify(
        &node,
        Input::Text("```json\n1.0\n```"),
        &json!({}),
        &options,
        &selta_core::Runtime::new(&registry, &NoCache),
    )
    .await;
    assert_eq!(fenced.verdict, Verdict::Fail);

    let coerced = common::run(&node, json!("1.7"), json!({}), &options, &registry).await;
    assert_eq!(coerced.verdict, Verdict::Fail, "no string→number coercion in strict mode");
}

#[tokio::test]
async fn lenient_mode_coerces_numeric_strings_with_a_notice() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "float",
        "verify": [ { "ext": "range", "config": { "min": 0.0, "max": 1.0 } } ]
    }));
    let report = common::run(&node, json!("0.7"), json!({}), &Options::default(), &registry).await;

    assert_eq!(report.verdict, Verdict::Pass, "coerced value flows into verifiers");
    assert!(report.notices.iter().any(|n| n.message.contains("coerced")));
}

// --- structural semantics (docs/02) ---

#[tokio::test]
async fn closed_objects_report_unexpected_and_missing_keys() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({ "type": "object", "fields": { "a": { "type": "int" } } }));
    let options = Options::default();

    let extra = common::run(&node, json!({ "a": 1, "b": 2 }), json!({}), &options, &registry).await;
    assert_eq!(extra.verdict, Verdict::Fail);
    assert!(extra.deltas[0].message.contains("unexpected key 'b'"));

    let missing = common::run(&node, json!({}), json!({}), &options, &registry).await;
    assert_eq!(missing.verdict, Verdict::Fail);
    assert_eq!(missing.deltas[0].path, "$.a");
    assert!(missing.deltas[0].message.contains("missing required key"));
}

#[tokio::test]
async fn union_reports_the_fewest_delta_variant() {
    let registry = Registry::with_builtins(None);
    let node = schema(json!({
        "type": "union",
        "variants": [
            { "type": "object", "fields": { "x": { "type": "int" }, "y": { "type": "int" } } },
            { "type": "object", "fields": { "a": { "type": "int" }, "b": { "type": "str" } } }
        ]
    }));
    let report = common::run(
        &node,
        json!({ "a": 1, "b": 5 }),
        json!({}),
        &Options::default(),
        &registry,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Fail);
    assert_eq!(report.root.checks[0].variant, Some(1), "best-match variant recorded");
    assert_eq!(report.deltas.len(), 1);
    assert_eq!(report.deltas[0].path, "$.b");
}

// --- caching: sound exactly for Dirac kernels (docs/08) ---

#[tokio::test]
async fn deterministic_results_are_cached_by_content() {
    let host = ScriptedHost::script(vec![pass(), pass()]);
    let registry = registry_with("det_check", Determinism::Deterministic, None, host.clone());
    let node = schema(json!({ "type": "str", "verify": [ { "ext": "det_check", "config": {} } ] }));
    let cache = MemoryCache::default();
    let options = Options::default();

    for _ in 0..2 {
        let report = selta_core::verify(
            &node,
            Input::Value(json!("same value")),
            &json!({}),
            &options,
            &selta_core::Runtime::new(&registry, &cache),
        )
        .await;
        assert_eq!(report.verdict, Verdict::Pass);
    }
    assert_eq!(host.calls(), 1, "second verification came from the cache");
}

// --- meta-validation (docs/02 §Meta-validation) ---

#[test]
fn meta_validation_catches_catalog_level_mistakes() {
    let registry = Registry::with_builtins(None);

    let unknown = schema(json!({ "type": "str", "verify": [ { "ext": "no_such_ext" } ] }));
    let errors = selta_core::meta::validate(&unknown, &registry);
    assert!(errors.iter().any(|e| e.contains("unknown extension")));

    let sampled_deterministic = schema(json!({
        "type": "str",
        "verify": [ { "ext": "regex", "config": { "pattern": "x" },
                      "sampling": { "samples": 3 } } ]
    }));
    let errors = selta_core::meta::validate(&sampled_deterministic, &registry);
    assert!(errors.iter().any(|e| e.contains("sampling on deterministic")));

    let bad_config = schema(json!({
        "type": "str",
        "verify": [ { "ext": "regex", "config": { "pattern": 42 } } ]
    }));
    let errors = selta_core::meta::validate(&bad_config, &registry);
    assert!(errors.iter().any(|e| e.contains("config_schema")));
}
