//! End-to-end over HTTP (docs/07 M4 acceptance): a real router, real node
//! hosts over stdio and websocket, real settings resolution with secrets.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::RwLock;

use selta_core::{MemoryCache, Registry, RpcHost};
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

    let request = json!({ "schema": "answer", "value": "an answer", "options": { "wait_ms": 30000 } });
    let (status, report) = http("POST", format!("{base}/pools/app/verify"), Some(request.clone())).await;
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
    assert!(judge["calls"].as_u64().expect("calls counted") >= 6, "{stats}");
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
    let base = serve(state).await;

    let (status, _) = http("POST", format!("{base}/pools"), Some(json!({ "name": "wspool" }))).await;
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
        body["extensions"]
            .as_array()
            .is_some_and(|items| items.iter().any(|i| i["name"] == "parity" && i["available"] == true))
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

    let request = |n: i64| json!({ "schema": "evenness", "value": n, "options": { "wait_ms": 10000 } });
    let (_, report) = http("POST", format!("{base}/pools/wspool/verify"), Some(request(3))).await;
    assert_eq!(report["verdict"], "fail", "{report}");
    assert!(report["deltas"][0]["message"]
        .as_str()
        .expect("delta message")
        .contains("even number"));
    let (_, report) = http("POST", format!("{base}/pools/wspool/verify"), Some(request(4))).await;
    assert_eq!(report["verdict"], "pass", "{report}");

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

    let (_, report) = http("POST", format!("{base}/pools/wspool/verify"), Some(request(4))).await;
    assert_eq!(report["verdict"], "inconclusive", "{report}");
    assert_eq!(report["deltas"], json!([]), "disconnection is never a delta");
}
