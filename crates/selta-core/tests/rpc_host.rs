//! M3 acceptance: the worked flow end-to-end with a real child process host
//! over ndjson JSON-RPC. Skips when node is unavailable.

mod common;

use selta_core::host::{AssessmentEnvelope, ExtensionHost, HostCall};
use selta_core::verdict::EvidenceState;
use selta_core::{EffectClass, InputKind, Options, Registry, RpcHost, Verdict};
use serde_json::json;

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

async fn assess(
    host: &RpcHost,
    ext: &str,
    value: &serde_json::Value,
    empty: &serde_json::Value,
) -> AssessmentEnvelope {
    host.assess(HostCall {
        ext,
        config: empty,
        settings: empty,
        value,
        path: "$",
        root: None,
        env: None,
        depth: 1,
        deadline_ms: Some(2_000),
    })
    .await
    .expect("RPC assessment")
}

#[tokio::test]
async fn ts_host_end_to_end() {
    if !node_available() {
        eprintln!("skipping ts_host_end_to_end: node not found");
        return;
    }
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/host.mjs");
    let (decls, host) = RpcHost::spawn(
        &["node".to_string(), fixture.to_string()],
        "selta-core-test",
    )
    .await
    .expect("host spawns and initializes");

    let length_judge = decls
        .iter()
        .find(|declaration| declaration.name == "length_judge")
        .expect("legacy declaration propagated");
    assert_eq!(
        length_judge.semantic_revision,
        "selta.extension.unversioned"
    );
    assert!(!length_judge.cacheable, "legacy manifests fail safe");
    assert_eq!(length_judge.effect_class, EffectClass::Unknown);
    assert_eq!(length_judge.accepted_input.kinds(), InputKind::ALL);
    let always_pass = decls
        .iter()
        .find(|declaration| declaration.name == "always_pass")
        .expect("versioned declaration propagated");
    assert_eq!(always_pass.semantic_revision, "selta.test.always_pass.v1");
    assert!(always_pass.cacheable);
    assert_eq!(always_pass.effect_class, EffectClass::Pure);
    assert_eq!(always_pass.accepted_input.kinds(), &[InputKind::Str]);

    let mut registry = Registry::with_builtins(None);
    registry
        .register(decls, host.clone())
        .expect("register host extensions");

    let schema = common::schema(json!({
        "type": "str",
        "verify": [
            { "ext": "always_pass", "config": {} },
            { "ext": "length_judge", "config": { "min": 5 },
              "sampling": { "samples": 3, "vote": "unanimous", "depth": 1 } }
        ]
    }));
    let options = Options::default();

    let ok = common::run(
        &schema,
        json!("long enough"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(ok.verdict, Verdict::Pass);
    assert_eq!(
        ok.usage.samples, 4,
        "one deterministic call + three judge samples"
    );

    let short = common::run(&schema, json!("hi"), json!({}), &options, &registry).await;
    assert_eq!(short.verdict, Verdict::Fail);
    assert!(short.deltas[0].message.contains("too short"));
    assert_eq!(short.deltas[0].votes.expect("tally").fail, 3);

    let empty = json!({});
    let conflict = json!("conflicted");
    let native = assess(&host, "length_judge", &conflict, &empty).await;
    assert_eq!(native.state(), EvidenceState::Both);
    assert!(native
        .refute
        .expect("conflict refutation")
        .message
        .contains("both polarities"));

    let supported = assess(&host, "length_judge", &json!("long enough"), &empty).await;
    assert_eq!(supported.state(), EvidenceState::SupportOnly);
    let refuted = assess(&host, "length_judge", &json!("hi"), &empty).await;
    assert_eq!(refuted.state(), EvidenceState::RefuteOnly);
    let abstained = assess(&host, "length_judge", &json!("uncertain"), &empty).await;
    assert_eq!(abstained.state(), EvidenceState::Neither);

    let legacy = json!("anything");
    let embedded = assess(&host, "always_pass", &legacy, &empty).await;
    assert_eq!(embedded.state(), EvidenceState::SupportOnly);

    // Transport accepts the closed envelope shape, then the engine performs
    // usage and refutation admission. Neither native error may trigger the
    // method-not-found legacy fallback (the legacy verifier would pass both).
    let evidence_schema = common::schema(json!({
        "type": "str",
        "verify": [{
            "ext": "length_judge",
            "evidence": "cautious",
            "sampling": { "samples": 1, "min_valid": 1 }
        }]
    }));
    for invalid in ["blank refutation", "invalid usage"] {
        let report = common::run(
            &evidence_schema,
            json!(invalid),
            json!({}),
            &Options::default(),
            &registry,
        )
        .await;
        assert_eq!(
            report.verdict,
            Verdict::Inconclusive,
            "{invalid}: {report:?}"
        );
        assert!(report.deltas.is_empty());
        assert_eq!(report.errors.len(), 1);
        let check = report
            .root
            .checks
            .iter()
            .find(|check| check.evidence.is_some())
            .expect("evidence check");
        let summary = check.evidence.as_ref().unwrap();
        assert_eq!(summary.state, None);
        assert_eq!(summary.unavailable, 2, "one attempt plus one bounded retry");
    }

    host.shutdown().await;
}
