//! M3 acceptance: the worked flow end-to-end with a real child process host
//! over ndjson JSON-RPC. Skips when node is unavailable.

mod common;

use serde_json::json;
use selta_core::{Options, Registry, RpcHost, Verdict};

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

    assert!(decls.iter().any(|d| d.name == "length_judge"));
    assert!(decls.iter().any(|d| d.name == "always_pass"));

    let mut registry = Registry::with_builtins(None);
    registry.register(decls, host.clone()).expect("register host extensions");

    let schema = common::schema(json!({
        "type": "str",
        "verify": [
            { "ext": "always_pass", "config": {} },
            { "ext": "length_judge", "config": { "min": 5 },
              "sampling": { "samples": 3, "vote": "unanimous", "depth": 1 } }
        ]
    }));
    let options = Options::default();

    let ok = common::run(&schema, json!("long enough"), json!({}), &options, &registry).await;
    assert_eq!(ok.verdict, Verdict::Pass);
    assert_eq!(ok.usage.samples, 4, "one deterministic call + three judge samples");

    let short = common::run(&schema, json!("hi"), json!({}), &options, &registry).await;
    assert_eq!(short.verdict, Verdict::Fail);
    assert!(short.deltas[0].message.contains("too short"));
    assert_eq!(short.deltas[0].votes.expect("tally").fail, 3);

    host.shutdown().await;
}
