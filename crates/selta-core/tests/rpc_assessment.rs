use std::sync::Arc;
use std::time::Duration;

use selta_core::host::rpc::{assess_over_peer, RpcPeer};
use selta_core::host::HostCall;
use selta_core::verdict::EvidenceState;
use selta_protocol::{Request, CODE_INVALID_PARAMS, CODE_UNKNOWN, CODE_VERIFIER_FAILED};
use serde_json::{json, Value};
use tokio::task::JoinHandle;

fn spawn_assess(
    peer: Arc<RpcPeer>,
    value: Value,
) -> JoinHandle<Result<selta_core::host::AssessmentEnvelope, String>> {
    tokio::spawn(async move {
        let empty = json!({});
        assess_over_peer(
            &peer,
            HostCall {
                ext: "judge",
                config: &empty,
                settings: &empty,
                value: &value,
                path: "$.answer",
                root: None,
                env: None,
                depth: 1,
                deadline_ms: Some(1_000),
            },
        )
        .await
    })
}

async fn next_request(outbox: &mut tokio::sync::mpsc::UnboundedReceiver<String>) -> Request {
    let line = outbox.recv().await.expect("outgoing request");
    serde_json::from_str(&line).expect("request parses")
}

fn respond(peer: &RpcPeer, id: u64, result: Value) {
    peer.accept_line(&json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string());
}

fn respond_error(peer: &RpcPeer, id: u64, code: i64, message: &str) {
    peer.accept_line(
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        })
        .to_string(),
    );
}

#[tokio::test]
async fn native_assess_reuses_verify_params_and_preserves_conflict() {
    let (peer, mut outbox) = RpcPeer::new();
    let task = spawn_assess(peer.clone(), json!("ambiguous"));

    let request = next_request(&mut outbox).await;
    assert_eq!(request.method, "assess");
    assert_eq!(request.params["ext"], "judge");
    assert_eq!(request.params["value"], "ambiguous");
    assert_eq!(request.params["path"], "$.answer");
    assert_eq!(request.params["budget"]["depth"], 1);
    respond(
        &peer,
        request.id.expect("request id"),
        json!({
            "support": true,
            "refute": { "message": "a counter-reading remains" },
            "usage": { "input_tokens": 8, "output_tokens": 3 }
        }),
    );

    let assessment = task.await.expect("task joins").expect("native assessment");
    assert_eq!(assessment.state(), EvidenceState::Both);
    assert_eq!(
        assessment.refute.expect("refutation").message,
        "a counter-reading remains"
    );
    assert_eq!(assessment.usage.expect("usage").input_tokens, 8);
}

#[tokio::test]
async fn method_not_found_alone_falls_back_to_one_legacy_verify() {
    let (peer, mut outbox) = RpcPeer::new();
    let task = spawn_assess(peer.clone(), json!("legacy"));

    let assess = next_request(&mut outbox).await;
    assert_eq!(assess.method, "assess");
    respond_error(
        &peer,
        assess.id.expect("assess id"),
        CODE_UNKNOWN,
        "unknown method",
    );

    let verify = next_request(&mut outbox).await;
    assert_eq!(verify.method, "verify");
    assert_eq!(verify.params, assess.params, "fallback reuses exact params");
    respond(
        &peer,
        verify.id.expect("verify id"),
        json!({
            "verdict": "fail",
            "delta": { "message": "legacy rejection" },
            "usage": { "output_tokens": 2 }
        }),
    );

    let assessment = task.await.expect("task joins").expect("legacy embedding");
    assert_eq!(assessment.state(), EvidenceState::RefuteOnly);
    assert_eq!(
        assessment.refute.expect("refutation").message,
        "legacy rejection"
    );
    assert_eq!(assessment.usage.expect("usage").output_tokens, 2);
}

#[tokio::test]
async fn operational_rpc_errors_never_trigger_legacy_fallback() {
    let (peer, mut outbox) = RpcPeer::new();
    let task = spawn_assess(peer.clone(), json!("broken"));

    let assess = next_request(&mut outbox).await;
    respond_error(
        &peer,
        assess.id.expect("assess id"),
        CODE_VERIFIER_FAILED,
        "upstream unavailable",
    );

    let error = task
        .await
        .expect("task joins")
        .expect_err("assessment must fail");
    assert_eq!(error, "host error -32000: upstream unavailable");
    assert!(
        tokio::time::timeout(Duration::from_millis(50), outbox.recv())
            .await
            .is_err(),
        "a non-method error must not dispatch verify"
    );
}

#[tokio::test]
async fn malformed_assessment_never_triggers_legacy_fallback() {
    let (peer, mut outbox) = RpcPeer::new();
    let task = spawn_assess(peer.clone(), json!("malformed"));

    let assess = next_request(&mut outbox).await;
    respond(
        &peer,
        assess.id.expect("assess id"),
        json!({ "refute": { "message": "support is required" } }),
    );

    let error = task
        .await
        .expect("task joins")
        .expect_err("malformed assessment must fail");
    assert!(error.contains("malformed assessment envelope"), "{error}");
    assert!(
        tokio::time::timeout(Duration::from_millis(50), outbox.recv())
            .await
            .is_err(),
        "malformed success is not method-not-found"
    );
}

#[tokio::test]
async fn public_rpc_call_keeps_its_string_error_contract() {
    let (peer, mut outbox) = RpcPeer::new();
    let task_peer = peer.clone();
    let task = tokio::spawn(async move {
        task_peer
            .call("custom", json!({}), 1_000)
            .await
            .expect_err("remote error")
    });

    let request = next_request(&mut outbox).await;
    respond_error(
        &peer,
        request.id.expect("request id"),
        CODE_INVALID_PARAMS,
        "bad custom input",
    );
    assert_eq!(
        task.await.expect("task joins"),
        "host error -32602: bad custom input"
    );
}
