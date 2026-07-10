//! The Lean host against the real Lean toolchain — no fakes. The compiler
//! tests need `lean` (install via elan); the judge test calls the real OpenAI
//! API and is gated on OPENAI_API_KEY being set.

mod common;

use std::sync::Arc;

use serde_json::json;
use selta_core::{Determinism, Options, Registry, RpcHost, Verdict};

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn find_lean() -> Option<String> {
    if let Ok(explicit) = std::env::var("LEAN_BIN") {
        return Some(explicit);
    }
    let on_path = std::process::Command::new("lean")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if on_path {
        return Some("lean".to_string());
    }
    let home = std::env::var("HOME").ok()?;
    let elan = std::path::PathBuf::from(home).join(".elan/bin/lean");
    elan.exists()
        .then(|| elan.to_string_lossy().into_owned())
}

async fn spawn_lean_host() -> Option<(Registry, Arc<RpcHost>)> {
    if !node_available() {
        eprintln!("skipping: node not found");
        return None;
    }
    let Some(lean) = find_lean() else {
        eprintln!("skipping: lean not installed (install the real toolchain via elan)");
        return None;
    };
    std::env::set_var("LEAN_BIN", &lean);
    let lean_host_js = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/lean-host/index.js"
    );
    let (decls, host) = RpcHost::spawn(
        &["node".to_string(), lean_host_js.to_string()],
        "selta-lean-test",
    )
    .await
    .expect("lean host spawns and initializes");

    let check = decls.iter().find(|d| d.name == "lean_check").expect("lean_check declared");
    assert_eq!(check.determinism, Determinism::Deterministic);
    assert!(check.delta_schema.is_some(), "compiler deltas are typed");
    let reflects = decls.iter().find(|d| d.name == "lean_reflects").expect("lean_reflects declared");
    assert_eq!(reflects.determinism, Determinism::Nondeterministic);

    let mut registry = Registry::with_builtins(None);
    registry.register(decls, host.clone()).expect("register lean extensions");
    Some((registry, host))
}

#[tokio::test]
async fn lean_check_with_real_lean() {
    let Some((registry, host)) = spawn_lean_host().await else {
        return;
    };
    let schema = common::schema(json!({
        "type": "str",
        "verify": [ { "ext": "lean_check", "config": {} } ]
    }));
    let options = Options::default();

    // A real proof, really compiled.
    let good = common::run(
        &schema,
        json!("theorem add_zero_id (n : Nat) : n + 0 = n := rfl"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(good.verdict, Verdict::Pass, "errors: {:?}", good.errors);

    // Unknown identifier → typed, structured delta from real diagnostics
    // (lean 4.31 writes "error(lean.unknownIdentifier): ...").
    let unknown = common::run(
        &schema,
        json!("def broken := undefined_thing_xyz"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(unknown.verdict, Verdict::Fail);
    let delta = &unknown.deltas[0];
    assert_eq!(delta.source, "lean_check");
    assert!(
        delta.message.to_lowercase().contains("unknown identifier"),
        "real lean said: {}",
        delta.message
    );
    let errors = &delta.data.as_ref().expect("typed structured data")["errors"];
    assert_eq!(errors[0]["severity"], "error");
    assert_eq!(errors[0]["line"], 1);

    // Multi-error output with multi-line messages parses into several entries.
    let multi = common::run(
        &schema,
        json!("theorem broken : BAD := rfl"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(multi.verdict, Verdict::Fail);
    let entries = multi.deltas[0].data.as_ref().unwrap()["errors"]
        .as_array()
        .unwrap()
        .len();
    assert!(entries >= 2, "expected several diagnostics, got {entries}");

    // `sorry` really compiles (warning only) — verification fails it anyway.
    let lazy = common::run(
        &schema,
        json!("theorem lazy (n : Nat) : n + 0 = n := by sorry"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(lazy.verdict, Verdict::Fail);
    assert!(lazy.deltas[0].message.contains("sorry"));

    // ... unless the schema opts in.
    let permissive = common::schema(json!({
        "type": "str",
        "verify": [ { "ext": "lean_check", "config": { "allow_sorry": true } } ]
    }));
    let allowed = common::run(
        &permissive,
        json!("theorem lazy (n : Nat) : n + 0 = n := by sorry"),
        json!({}),
        &options,
        &registry,
    )
    .await;
    assert_eq!(allowed.verdict, Verdict::Pass);

    host.shutdown().await;
}

#[tokio::test]
async fn lean_reflects_with_real_codex() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!(
            "skipping lean_reflects_with_real_codex: OPENAI_API_KEY not set — \
             the real judge needs a real key (no stubs)"
        );
        return;
    }
    let Some((registry, host)) = spawn_lean_host().await else {
        return;
    };
    let schema = common::schema(json!({
        "type": "str",
        "verify": [
            { "ext": "lean_check", "config": {} },
            { "ext": "lean_reflects",
              "config": { "statement": { "$env": "statement" } },
              "sampling": { "samples": 1, "depth": 1 } }
        ]
    }));
    // xhigh reasoning can be slow; give the judge real room.
    let options = Options {
        deadline_ms: Some(300_000),
        ..Options::default()
    };
    let env = json!({ "statement": "For every natural number n, n + 0 = n." });

    let faithful = common::run(
        &schema,
        json!("theorem add_zero_id (n : Nat) : n + 0 = n := rfl"),
        env.clone(),
        &options,
        &registry,
    )
    .await;
    assert_eq!(faithful.verdict, Verdict::Pass, "errors: {:?}", faithful.errors);
    assert!(
        faithful.usage.input_tokens > 0,
        "real tokens were spent and reported"
    );

    // Compiles fine, but proves a vacuous reformulation — reflection must fail.
    let vacuous = common::run(
        &schema,
        json!("theorem vacuous (n : Nat) : n + 1 = n + 1 := rfl"),
        env,
        &options,
        &registry,
    )
    .await;
    assert_eq!(vacuous.verdict, Verdict::Fail);
    assert!(vacuous.deltas.iter().any(|d| d.source == "lean_reflects"));

    host.shutdown().await;
}
