//! Cache eligibility and identity are declaration-level soundness contracts.

mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use selta_core::{
    CmdInput, CmdTemplate, CmdTemplates, Determinism, ExtensionDecl, MemoryCache, Needs, Node,
    Options, Registry, Runtime, Verdict,
};
use serde_json::{json, Value};

use common::{schema, ScriptedHost};

fn registry_with_decl(
    name: &str,
    semantic_revision: &str,
    cacheable: bool,
    determinism: Determinism,
    host: Arc<ScriptedHost>,
) -> Registry {
    let mut registry = Registry::with_builtins(None);
    registry
        .register(
            vec![ExtensionDecl {
                name: name.to_string(),
                semantic_revision: semantic_revision.to_string(),
                cacheable,
                determinism,
                config_schema: None,
                needs: Needs::default(),
                settings_schema: None,
                delta_schema: None,
            }],
            host,
        )
        .expect("test declaration registers");
    registry
}

async fn run(
    node: &Node,
    value: Value,
    options: &Options,
    registry: &Registry,
    cache: &MemoryCache,
) {
    let report = selta_core::verify(
        node,
        selta_core::Input::Value(value),
        &json!({}),
        options,
        &Runtime::new(registry, cache),
    )
    .await;
    assert_eq!(report.verdict, Verdict::Pass, "{report:?}");
}

#[tokio::test]
async fn cache_identity_separates_json_paths() {
    let host = ScriptedHost::script(vec![]);
    let registry = registry_with_decl(
        "path_sensitive",
        "selta.test.path_sensitive.v1",
        true,
        Determinism::Deterministic,
        host.clone(),
    );
    let node = schema(json!({
        "type": "object",
        "fields": {
            "left": { "type": "str", "verify": [{ "ext": "path_sensitive" }] },
            "right": { "type": "str", "verify": [{ "ext": "path_sensitive" }] }
        }
    }));
    let value = json!({ "left": "same", "right": "same" });
    let cache = MemoryCache::default();

    run(&node, value.clone(), &Options::default(), &registry, &cache).await;
    run(&node, value, &Options::default(), &registry, &cache).await;

    assert_eq!(host.calls(), 2, "one execution per distinct JSON path");
}

#[tokio::test]
async fn cache_identity_separates_recursion_depths() {
    let host = ScriptedHost::script(vec![]);
    let registry = registry_with_decl(
        "depth_sensitive",
        "selta.test.depth_sensitive.v1",
        true,
        Determinism::Deterministic,
        host.clone(),
    );
    let node = schema(json!({
        "type": "str",
        "verify": [{ "ext": "depth_sensitive" }]
    }));
    let cache = MemoryCache::default();

    for max_depth in [1, 2, 1] {
        let options = Options {
            max_depth,
            ..Options::default()
        };
        run(&node, json!("same"), &options, &registry, &cache).await;
    }

    assert_eq!(host.calls(), 2, "depth 1 and depth 2 use separate entries");
}

#[tokio::test]
async fn cache_identity_separates_semantic_revisions() {
    let first_host = ScriptedHost::script(vec![]);
    let second_host = ScriptedHost::script(vec![]);
    let first = registry_with_decl(
        "versioned",
        "selta.test.versioned.v1",
        true,
        Determinism::Deterministic,
        first_host.clone(),
    );
    let second = registry_with_decl(
        "versioned",
        "selta.test.versioned.v2",
        true,
        Determinism::Deterministic,
        second_host.clone(),
    );
    let node = schema(json!({
        "type": "str",
        "verify": [{ "ext": "versioned" }]
    }));
    let cache = MemoryCache::default();

    run(&node, json!("same"), &Options::default(), &first, &cache).await;
    run(&node, json!("same"), &Options::default(), &second, &cache).await;
    run(&node, json!("same"), &Options::default(), &first, &cache).await;

    assert_eq!(first_host.calls(), 1);
    assert_eq!(second_host.calls(), 1);
}

#[derive(Default)]
struct CountingCmdTemplates {
    lookups: AtomicU32,
}

impl CmdTemplates for CountingCmdTemplates {
    fn get(&self, name: &str) -> Option<CmdTemplate> {
        if name != "counted" {
            return None;
        }
        self.lookups.fetch_add(1, Ordering::SeqCst);
        Some(CmdTemplate {
            run: vec!["true".to_string()],
            input: CmdInput::File,
            timeout_ms: 1_000,
        })
    }
}

#[tokio::test]
async fn cmd_is_non_cacheable_even_with_a_shared_cache() {
    let templates = Arc::new(CountingCmdTemplates::default());
    let registry = Registry::with_builtins(Some(templates.clone()));
    assert!(!registry.decl("cmd").expect("cmd builtin").cacheable);
    let node = schema(json!({
        "type": "str",
        "verify": [{ "ext": "cmd", "config": { "name": "counted" } }]
    }));
    let cache = MemoryCache::default();

    run(&node, json!("same"), &Options::default(), &registry, &cache).await;
    run(&node, json!("same"), &Options::default(), &registry, &cache).await;

    assert_eq!(
        templates.lookups.load(Ordering::SeqCst),
        2,
        "cmd must execute rather than reuse its first process result"
    );
}

#[test]
fn registry_rejects_cacheable_nondeterministic_declarations() {
    let host = ScriptedHost::script(vec![]);
    let mut registry = Registry::with_builtins(None);
    let error = registry
        .register(
            vec![ExtensionDecl {
                name: "unsafe_cache".to_string(),
                semantic_revision: "selta.test.unsafe_cache.v1".to_string(),
                cacheable: true,
                determinism: Determinism::Nondeterministic,
                config_schema: None,
                needs: Needs::default(),
                settings_schema: None,
                delta_schema: None,
            }],
            host,
        )
        .expect_err("nondeterministic declarations cannot opt into caching");
    assert!(error.contains("only deterministic"), "{error}");
}

#[test]
fn registry_rejects_duplicate_batch_names_before_mutation() {
    let host = ScriptedHost::script(vec![]);
    let declaration = ExtensionDecl {
        name: "duplicate_batch".to_string(),
        semantic_revision: "selta.test.duplicate_batch.v1".to_string(),
        cacheable: false,
        determinism: Determinism::Deterministic,
        config_schema: None,
        needs: Needs::default(),
        settings_schema: None,
        delta_schema: None,
    };
    let mut registry = Registry::with_builtins(None);
    let error = registry
        .register(vec![declaration.clone(), declaration], host)
        .expect_err("duplicate names in one atomic batch must reject");

    assert!(error.contains("duplicate extension name"), "{error}");
    assert!(
        registry.decl("duplicate_batch").is_none(),
        "failed batch must not partially mutate the registry"
    );
}
