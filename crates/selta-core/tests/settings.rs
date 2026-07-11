//! Settings semantics (docs/05 §Three kinds of configuration): resolved
//! settings reach the host, fingerprints reach the report and the cache key,
//! and a settings_schema violation is inconclusive — never a delta.

mod common;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use selta_core::{
    Determinism, Envelope, ExtensionDecl, ExtensionHost, HostCall, MemoryCache, Needs, Node,
    Options, Registry, ResolvedSettings, Runtime, SettingsResolver, Verdict,
};
use serde_json::{json, Value};

use common::schema;

/// Records the settings each call arrived with; always passes.
#[derive(Default)]
struct CapturingHost {
    seen: Mutex<Vec<Value>>,
}

#[async_trait]
impl ExtensionHost for CapturingHost {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String> {
        self.seen.lock().unwrap().push(call.settings.clone());
        Ok(Envelope::pass())
    }
}

struct StaticSettings(Value);

impl SettingsResolver for StaticSettings {
    fn resolve(&self, _ext: &str) -> Result<ResolvedSettings, String> {
        Ok(ResolvedSettings {
            value: self.0.clone(),
            fingerprint: selta_core::settings::fingerprint(&self.0),
        })
    }
}

fn registry_with_decl(decl: ExtensionDecl, host: Arc<dyn ExtensionHost>) -> Registry {
    let mut registry = Registry::with_builtins(None);
    registry.register(vec![decl], host).expect("no name clash");
    registry
}

fn judged_str_schema(ext: &str) -> Node {
    schema(json!({ "type": "str", "verify": [ { "ext": ext, "config": {} } ] }))
}

#[tokio::test]
async fn settings_reach_the_host_and_fingerprint_reaches_the_report() {
    let host = Arc::new(CapturingHost::default());
    let registry = registry_with_decl(
        ExtensionDecl {
            name: "judge".to_string(),
            semantic_revision: "selta.test.judge.v1".to_string(),
            cacheable: false,
            determinism: Determinism::Deterministic,
            config_schema: None,
            needs: Needs::default(),
            settings_schema: None,
            delta_schema: None,
        },
        host.clone(),
    );
    let settings = StaticSettings(json!({ "model": "claude-sonnet-5" }));
    let runtime = Runtime {
        registry: &registry,
        cache: &selta_core::NoCache,
        settings: &settings,
        monitor: None,
    };

    let report = selta_core::verify(
        &judged_str_schema("judge"),
        selta_core::Input::Value(json!("value")),
        &json!({}),
        &Options::default(),
        &runtime,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(
        host.seen.lock().unwrap().as_slice(),
        &[json!({ "model": "claude-sonnet-5" })]
    );
    let fingerprint = report
        .extensions
        .get("judge")
        .expect("fingerprint recorded");
    assert_eq!(
        fingerprint,
        &selta_core::settings::fingerprint(&json!({ "model": "claude-sonnet-5" }))
    );
}

#[tokio::test]
async fn cache_keys_include_the_settings_fingerprint() {
    let host = Arc::new(CapturingHost::default());
    let registry = registry_with_decl(
        ExtensionDecl {
            name: "judge".to_string(),
            semantic_revision: "selta.test.judge.v1".to_string(),
            cacheable: true,
            determinism: Determinism::Deterministic,
            config_schema: None,
            needs: Needs::default(),
            settings_schema: None,
            delta_schema: None,
        },
        host.clone(),
    );
    let cache = MemoryCache::default();
    let node = judged_str_schema("judge");

    for model in ["model-a", "model-a", "model-b"] {
        let settings = StaticSettings(json!({ "model": model }));
        let runtime = Runtime {
            registry: &registry,
            cache: &cache,
            settings: &settings,
            monitor: None,
        };
        selta_core::verify(
            &node,
            selta_core::Input::Value(json!("same value")),
            &json!({}),
            &Options::default(),
            &runtime,
        )
        .await;
    }

    // Same settings hit the cache; different settings are a different key —
    // a verdict from one model is not a verdict from another (docs/03).
    assert_eq!(host.seen.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn settings_schema_violation_is_inconclusive_not_fail() {
    let host = Arc::new(CapturingHost::default());
    let registry = registry_with_decl(
        ExtensionDecl {
            name: "judge".to_string(),
            semantic_revision: "selta.test.judge.v1".to_string(),
            cacheable: false,
            determinism: Determinism::Deterministic,
            config_schema: None,
            needs: Needs::default(),
            settings_schema: Some(schema(json!({
                "type": "object", "open": true,
                "fields": { "model": { "type": "str" } }
            }))),
            delta_schema: None,
        },
        host.clone(),
    );
    let settings = StaticSettings(json!({}));
    let runtime = Runtime {
        registry: &registry,
        cache: &selta_core::NoCache,
        settings: &settings,
        monitor: None,
    };

    let report = selta_core::verify(
        &judged_str_schema("judge"),
        selta_core::Input::Value(json!("value")),
        &json!({}),
        &Options::default(),
        &runtime,
    )
    .await;

    assert_eq!(report.verdict, Verdict::Inconclusive);
    assert!(
        report.deltas.is_empty(),
        "misconfiguration is never a delta"
    );
    assert!(report.errors[0].error.contains("settings_schema"));
    assert!(host.seen.lock().unwrap().is_empty(), "host never called");
}
