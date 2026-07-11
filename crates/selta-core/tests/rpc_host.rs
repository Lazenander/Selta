//! M3 acceptance: the worked flow end-to-end with a real child process host
//! over ndjson JSON-RPC. Skips when node is unavailable.

mod common;

use selta_core::{Options, Registry, RpcHost, Verdict};
use serde_json::json;

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
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
    let always_pass = decls
        .iter()
        .find(|declaration| declaration.name == "always_pass")
        .expect("versioned declaration propagated");
    assert_eq!(always_pass.semantic_revision, "selta.test.always_pass.v1");
    assert!(always_pass.cacheable);

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

    host.shutdown().await;
}
