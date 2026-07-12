//! End-to-end over HTTP (docs/07 M4 acceptance): a real router, real node
//! hosts over stdio and websocket, real settings resolution with secrets.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::RwLock;

use selta_core::{EvidenceState, HostCall, MemoryCache, Registry, RpcHost};
use seltad::api;
use seltad::state::AppState;
use seltad::stats::Stats;
use seltad::storage::SqliteStorage;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn app_state(
    registry: Registry,
    server_settings: HashMap<String, Value>,
    dir: &tempfile::TempDir,
) -> Arc<AppState> {
    Arc::new(AppState {
        registry: Arc::new(registry),
        catalog: Arc::new(
            SqliteStorage::open(&dir.path().join("selta.db")).expect("catalog opens"),
        ),
        cache: Arc::new(MemoryCache::default()),
        server_settings: Arc::new(server_settings),
        stats: Arc::new(Stats::default()),
        semaphores: RwLock::new(HashMap::new()),
        jobs: RwLock::new(HashMap::new()),
        pool_hosts: RwLock::new(HashMap::new()),
    })
}

async fn serve(state: Arc<AppState>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, api::router(state))
            .await
            .expect("server runs");
    });
    format!("http://{addr}")
}

async fn http(method: &'static str, url: String, body: Option<Value>) -> (u16, Value) {
    tokio::task::spawn_blocking(move || {
        let result = match method {
            "GET" => ureq::get(&url).call(),
            "POST" => ureq::post(&url).send_json(body.expect("body")),
            "PUT" => ureq::put(&url).send_json(body.expect("body")),
            other => panic!("unsupported method {other}"),
        };
        match result {
            Ok(response) => {
                let status = response.status();
                (status, response.into_json().unwrap_or_else(|_| json!({})))
            }
            Err(ureq::Error::Status(status, response)) => {
                (status, response.into_json().unwrap_or_else(|_| json!({})))
            }
            Err(error) => panic!("http {method} {url}: {error}"),
        }
    })
    .await
    .expect("http task")
}

async fn put_raw(url: String, source: &str) -> (u16, Value) {
    let source = source.to_string();
    tokio::task::spawn_blocking(move || {
        let result = ureq::put(&url)
            .set("content-type", "application/json")
            .send_string(&source);
        match result {
            Ok(response) => {
                let status = response.status();
                (status, response.into_json().unwrap_or_else(|_| json!({})))
            }
            Err(ureq::Error::Status(status, response)) => {
                (status, response.into_json().unwrap_or_else(|_| json!({})))
            }
            Err(error) => panic!("http PUT {url}: {error}"),
        }
    })
    .await
    .expect("http task")
}

#[tokio::test]
async fn schema_registration_is_raw_strict_typed_and_atomic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = serve(app_state(
        Registry::with_builtins(None),
        HashMap::new(),
        &dir,
    ))
    .await;
    let (status, _) = http(
        "POST",
        format!("{base}/pools"),
        Some(json!({ "name": "app" })),
    )
    .await;
    assert_eq!(status, 201);

    let (status, duplicate) = put_raw(
        format!("{base}/pools/app/schemas/strict"),
        r#"{ "type": "str", "type": "int" }"#,
    )
    .await;
    assert_eq!(status, 400, "{duplicate}");
    assert_eq!(duplicate["error"], "schema rejected by strict admission");
    assert_eq!(duplicate["issues"][0]["code"], "DUPLICATE_OBJECT_KEY");
    assert_eq!(duplicate["issues"][0]["pointer"], "/type");

    let (status, unknown) = put_raw(
        format!("{base}/pools/app/schemas/strict"),
        r#"{ "type": "str", "ignored": true }"#,
    )
    .await;
    assert_eq!(status, 400, "{unknown}");
    assert!(unknown["issues"].as_array().is_some_and(|issues| {
        issues
            .iter()
            .any(|issue| issue["code"] == "UNEXPECTED_PROPERTY" && issue["pointer"] == "/ignored")
    }));

    let (status, missing) = http("GET", format!("{base}/pools/app/schemas/strict"), None).await;
    assert_eq!(
        status, 404,
        "rejected sources must not allocate a version: {missing}"
    );

    // A valid dynamic hole does not excuse invalid literal siblings in the
    // same config object.
    let (status, sibling) = put_raw(
        format!("{base}/pools/app/schemas/strict"),
        r#"{
          "type": "str",
          "verify": [{
            "ext": "regex",
            "config": { "pattern": { "$env": "expected.pattern" }, "ignored": true }
          }]
        }"#,
    )
    .await;
    assert_eq!(status, 400, "{sibling}");
    assert!(sibling["issues"].as_array().is_some_and(|issues| {
        issues.iter().any(|issue| {
            issue["code"] == "CONFIG_STRUCTURE" && issue["pointer"] == "/verify/0/config/ignored"
        })
    }));

    let (status, accepted) = put_raw(
        format!("{base}/pools/app/schemas/strict"),
        r#"{
          "type": "str",
          "verify": [{
            "ext": "regex",
            "config": { "pattern": { "$env": "expected.pattern" } }
          }]
        }"#,
    )
    .await;
    assert_eq!(status, 200, "{accepted}");
    assert_eq!(accepted["version"], 1, "rejections consumed no versions");

    let (status, fetched) = http("GET", format!("{base}/pools/app/schemas/strict@1"), None).await;
    assert_eq!(status, 200, "{fetched}");
    assert_eq!(fetched["schema"]["type"], "str");
    assert_eq!(
        fetched["schema"]["verify"][0]["config"]["pattern"]["$env"],
        "expected.pattern"
    );
}

#[tokio::test]
async fn catalog_schema_bytes_are_strictly_read_before_get_or_verify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = serve(app_state(
        Registry::with_builtins(None),
        HashMap::new(),
        &dir,
    ))
    .await;
    let (status, _) = http(
        "POST",
        format!("{base}/pools"),
        Some(json!({ "name": "app" })),
    )
    .await;
    assert_eq!(status, 201);
    let (status, body) = put_raw(
        format!("{base}/pools/app/schemas/tampered"),
        r#"{ "type": "str" }"#,
    )
    .await;
    assert_eq!(status, 200, "{body}");

    let connection = rusqlite::Connection::open(dir.path().join("selta.db")).expect("db opens");
    connection
        .execute(
            "UPDATE schemas SET body = ?1 WHERE pool = 'app' AND name = 'tampered' AND version = 1",
            [r#"{ "type": "str", "type": "int" }"#],
        )
        .expect("catalog tamper");

    let assert_corruption = |status: u16, body: &Value| {
        assert_eq!(status, 500, "{body}");
        assert!(body["error"]
            .as_str()
            .is_some_and(|error| error.contains("failed strict admission")));
        assert_eq!(body["issues"][0]["code"], "DUPLICATE_OBJECT_KEY");
        assert_eq!(body["issues"][0]["pointer"], "/type");
    };

    let (status, body) = http("GET", format!("{base}/pools/app/schemas/tampered@1"), None).await;
    assert_corruption(status, &body);

    let (status, body) = http(
        "POST",
        format!("{base}/pools/app/verify"),
        Some(json!({ "schema": "tampered@1", "value": "hello" })),
    )
    .await;
    assert_corruption(status, &body);
}

/// M4 acceptance (docs/07): the worked judge is reconfigured at pool scope —
/// the schema is never touched — and the report pins the fingerprints.
#[tokio::test]
async fn pool_scope_settings_reconfigure_the_judge_without_touching_the_schema() {
    std::env::set_var("SELTA_TEST_KEY", "sk-test-secret");
    let mut registry = Registry::with_builtins(None);
    let command = vec!["node".to_string(), fixture("model_judge.mjs")];
    let (decls, host) = RpcHost::spawn(&command, "seltad-test")
        .await
        .expect("judge host spawns");
    registry.register(decls, host).expect("no clash");
    let server_settings = HashMap::from([(
        "model_judge".to_string(),
        json!({ "model": "strict-model", "api_key": { "$secret": "SELTA_TEST_KEY" } }),
    )]);
    let dir = tempfile::tempdir().expect("tempdir");
    let base = serve(app_state(registry, server_settings, &dir)).await;

    let (status, _) = http(
        "POST",
        format!("{base}/pools"),
        Some(json!({ "name": "app", "extensions": ["model_judge"] })),
    )
    .await;
    assert_eq!(status, 201);
    let (status, body) = http(
        "PUT",
        format!("{base}/pools/app/schemas/answer"),
        Some(json!({
            "type": "str",
            "verify": [ { "ext": "model_judge", "sampling": { "samples": 3, "depth": 1 } } ]
        })),
    )
    .await;
    assert_eq!(status, 200, "{body}");

    let request =
        json!({ "schema": "answer", "value": "an answer", "options": { "wait_ms": 30000 } });
    let (status, report) = http(
        "POST",
        format!("{base}/pools/app/verify"),
        Some(request.clone()),
    )
    .await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(report["verdict"], "fail", "{report}");
    assert!(report["deltas"][0]["message"]
        .as_str()
        .expect("delta message")
        .contains("strict-model"));
    let strict_fingerprint = report["extensions"]["model_judge"]
        .as_str()
        .expect("fingerprint recorded")
        .to_string();

    let (status, _) = http(
        "PUT",
        format!("{base}/pools/app/settings/model_judge"),
        Some(json!({ "model": "lenient-model" })),
    )
    .await;
    assert_eq!(status, 200);
    let (status, report) = http("POST", format!("{base}/pools/app/verify"), Some(request)).await;
    assert_eq!(status, 200);
    assert_eq!(report["verdict"], "pass", "{report}");
    let lenient_fingerprint = report["extensions"]["model_judge"]
        .as_str()
        .expect("fingerprint recorded");
    assert_ne!(strict_fingerprint, lenient_fingerprint);

    // The monitoring surface saw every call (docs/06 §Monitoring).
    let (status, stats) = http("GET", format!("{base}/pools/app/stats"), None).await;
    assert_eq!(status, 200);
    let judge = &stats["extensions"]["model_judge"];
    assert!(
        judge["calls"].as_u64().expect("calls counted") >= 6,
        "{stats}"
    );
    assert_eq!(judge["agreement"], json!(1.0), "unanimous rounds");

    // Redaction: API responses echo the reference, never the secret value.
    let (_, extensions) = http("GET", format!("{base}/pools/app/extensions"), None).await;
    let text = extensions.to_string();
    assert!(text.contains("$secret:SELTA_TEST_KEY"), "{text}");
    assert!(!text.contains("sk-test-secret"), "{text}");
}

/// Pool hosts (docs/05 §websocket): dial in with no SDK, get implicitly
/// granted, serve verifies; a disconnect makes checks inconclusive — never
/// failures.
#[tokio::test]
async fn pool_host_dials_in_serves_verifies_and_disconnect_is_inconclusive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let state = app_state(Registry::with_builtins(None), HashMap::new(), &dir);
    let base = serve(state.clone()).await;

    let (status, _) = http(
        "POST",
        format!("{base}/pools"),
        Some(json!({ "name": "wspool" })),
    )
    .await;
    assert_eq!(status, 201);

    let ws_url = format!(
        "{}/pools/wspool/hosts/connect",
        base.replace("http://", "ws://")
    );
    let mut child = tokio::process::Command::new("node")
        .arg(fixture("pool_host.mjs"))
        .arg(&ws_url)
        .kill_on_drop(true)
        .spawn()
        .expect("pool host spawns");

    let listed = |body: &Value| {
        body["extensions"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|i| i["name"] == "parity" && i["available"] == true)
        })
    };
    let mut connected = false;
    for _ in 0..100 {
        let (_, body) = http("GET", format!("{base}/pools/wspool/extensions"), None).await;
        if listed(&body) {
            connected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(connected, "pool host never connected");

    // Implicit grant: 'parity' appears in no pool.extensions list — dialing
    // it in was the pool's own act.
    let (status, body) = http(
        "PUT",
        format!("{base}/pools/wspool/schemas/evenness"),
        Some(json!({ "type": "int", "verify": [ { "ext": "parity" } ] })),
    )
    .await;
    assert_eq!(status, 200, "{body}");

    let request =
        |n: i64| json!({ "schema": "evenness", "value": n, "options": { "wait_ms": 10000 } });
    let (_, report) = http(
        "POST",
        format!("{base}/pools/wspool/verify"),
        Some(request(3)),
    )
    .await;
    assert_eq!(report["verdict"], "fail", "{report}");
    assert!(report["deltas"][0]["message"]
        .as_str()
        .expect("delta message")
        .contains("even number"));
    let (_, report) = http(
        "POST",
        format!("{base}/pools/wspool/verify"),
        Some(request(4)),
    )
    .await;
    assert_eq!(report["verdict"], "pass", "{report}");

    // The same real WebSocket peer carries all four native assessment states.
    // A declaration without a native assessor returns structured -32601 and
    // falls back to its legacy verifier over that same transport.
    let registry = state.effective_registry("wspool").await;
    let (_, native_host) = registry.get("parity").expect("native pool host");
    let empty = json!({});
    for (value, expected) in [
        (json!("neither"), EvidenceState::Neither),
        (json!(4), EvidenceState::SupportOnly),
        (json!(3), EvidenceState::RefuteOnly),
        (json!("both"), EvidenceState::Both),
    ] {
        let assessment = native_host
            .assess(HostCall {
                ext: "parity",
                config: &empty,
                settings: &empty,
                value: &value,
                path: "$",
                root: None,
                env: None,
                depth: 1,
                deadline_ms: Some(2_000),
            })
            .await
            .expect("native WebSocket assessment");
        assert_eq!(assessment.state(), expected);
    }
    let (_, legacy_host) = registry
        .get("legacy_parity")
        .expect("legacy pool host declaration");
    let legacy_value = json!(4);
    let embedded = legacy_host
        .assess(HostCall {
            ext: "legacy_parity",
            config: &empty,
            settings: &empty,
            value: &legacy_value,
            path: "$",
            root: None,
            env: None,
            depth: 1,
            deadline_ms: Some(2_000),
        })
        .await
        .expect("WebSocket method-not-found fallback");
    assert_eq!(embedded.state(), EvidenceState::SupportOnly);

    child.kill().await.expect("kill pool host");
    let mut gone = false;
    for _ in 0..100 {
        let (_, body) = http("GET", format!("{base}/pools/wspool/extensions"), None).await;
        if !listed(&body) {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(gone, "pool host entry never cleaned up");

    let (_, report) = http(
        "POST",
        format!("{base}/pools/wspool/verify"),
        Some(request(4)),
    )
    .await;
    assert_eq!(report["verdict"], "inconclusive", "{report}");
    assert_eq!(
        report["deltas"],
        json!([]),
        "disconnection is never a delta"
    );

    let disconnected_value = json!(4);
    let error = native_host
        .assess(HostCall {
            ext: "parity",
            config: &empty,
            settings: &empty,
            value: &disconnected_value,
            path: "$",
            root: None,
            env: None,
            depth: 1,
            deadline_ms: Some(200),
        })
        .await
        .expect_err("a disconnected host cannot manufacture evidence");
    assert!(
        error.contains("host is gone") || error.contains("disconnected"),
        "{error}"
    );
}
