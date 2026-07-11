use std::sync::Arc;

use selta_core::{decl_from_manifest, initialize_over_peer, RpcPeer, Type};
use selta_protocol::{ExtensionManifest, Request};
use serde_json::value::RawValue;

fn raw(source: &str) -> Box<RawValue> {
    RawValue::from_string(source.to_string()).expect("test raw JSON")
}

fn manifest() -> ExtensionManifest {
    ExtensionManifest {
        name: "judge".to_string(),
        determinism: "deterministic".to_string(),
        semantic_revision: Some("selta.test.judge.v1".to_string()),
        cacheable: false,
        effect_class: Some("unknown".to_string()),
        accepted_input: Some(vec!["str".to_string()]),
        config_schema: None,
        needs: Vec::new(),
        settings_schema: None,
        delta_schema: None,
    }
}

#[test]
fn raw_manifest_schema_rejects_duplicate_keys() {
    let mut manifest = manifest();
    manifest.config_schema = Some(raw(r#"{"type":"object","type":"str"}"#));

    let error = decl_from_manifest(manifest).expect_err("duplicate schema key must fail");
    assert!(error.contains("DUPLICATE_OBJECT_KEY"), "{error}");
}

#[test]
fn declaration_needs_are_a_closed_duplicate_free_set() {
    for needs in [
        vec!["network".to_string()],
        vec!["root".to_string(), "root".to_string()],
        vec!["".to_string()],
    ] {
        let mut manifest = manifest();
        manifest.needs = needs;
        assert!(decl_from_manifest(manifest).is_err());
    }

    let mut manifest = manifest();
    manifest.needs = vec!["root".to_string(), "env".to_string()];
    let declaration = decl_from_manifest(manifest).expect("closed needs accepted");
    assert!(declaration.needs.root);
    assert!(declaration.needs.env);
}

#[test]
fn manifest_wire_shape_is_closed() {
    let source = r#"{
        "name":"judge",
        "determinism":"deterministic",
        "cacheable":false,
        "needs":[],
        "surprise":true
    }"#;
    assert!(serde_json::from_str::<ExtensionManifest>(source).is_err());
}

#[test]
fn valid_raw_declaration_schema_projects_only_after_strict_admission() {
    let mut manifest = manifest();
    manifest.config_schema = Some(raw(
        r#"{"type":"object","fields":{"pattern":{"type":"str"}}}"#,
    ));

    let declaration = decl_from_manifest(manifest).expect("strict schema accepted");
    assert!(matches!(
        declaration.config_schema.expect("config schema").ty,
        Type::Object { .. }
    ));
}

#[tokio::test]
async fn initialize_keeps_nested_schema_bytes_raw_until_admission() {
    let (peer, mut outbox) = RpcPeer::new();
    let task_peer = Arc::clone(&peer);
    let initialize =
        tokio::spawn(async move { initialize_over_peer(&task_peer, "test-server").await });

    let outgoing = outbox.recv().await.expect("initialize request");
    let request: Request = serde_json::from_str(&outgoing).expect("request parses");
    let id = request.id.expect("request id");
    let response = r#"{"jsonrpc":"2.0","id":__ID__,"result":{
            "host":{"name":"fixture","version":"1"},
            "extensions":[{
                "name":"judge",
                "determinism":"deterministic",
                "semantic_revision":"selta.test.judge.v1",
                "cacheable":false,
                "effect_class":"unknown",
                "accepted_input":["str"],
                "config_schema":{"type":"object","type":"str"},
                "needs":[]
            }]
        }}"#
    .replace("__ID__", &id.to_string());
    peer.accept_line(&response);

    let error = initialize
        .await
        .expect("initialize task joins")
        .expect_err("duplicate nested schema must fail");
    assert!(error.contains("DUPLICATE_OBJECT_KEY"), "{error}");
}

#[tokio::test]
async fn dropping_a_call_removes_it_and_notifies_the_host() {
    let (peer, mut outbox) = RpcPeer::new();
    let task_peer = Arc::clone(&peer);
    let call = tokio::spawn(async move {
        task_peer
            .call("verify", serde_json::json!({}), 60_000)
            .await
    });

    let outgoing = outbox.recv().await.expect("verify request");
    let request: Request = serde_json::from_str(&outgoing).expect("request parses");
    let id = request.id.expect("request id");
    call.abort();
    let _ = call.await;

    let cancel = tokio::time::timeout(std::time::Duration::from_secs(1), outbox.recv())
        .await
        .expect("cancel arrives promptly")
        .expect("cancel message");
    let cancel: Request = serde_json::from_str(&cancel).expect("cancel parses");
    assert_eq!(cancel.method, "cancel");
    assert_eq!(cancel.id, None);
    assert_eq!(cancel.params["id"], id);
}
