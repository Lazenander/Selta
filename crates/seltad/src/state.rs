//! Shared server state: registry, catalog, settings, stats, per-pool
//! semaphores, jobs, dialed-in pool hosts — and the pool-level registration
//! checks (enablement, literal cmd names).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use selta_core::{meta, ExtensionHost, MemoryCache, Node, Registry, Report, Type, VerifierSpec};
use serde_json::Value;
use tokio::sync::{watch, Mutex, RwLock, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::stats::Stats;
use crate::storage::{PoolConfig, Storage};
use crate::ws::PoolHosts;

pub struct AppState {
    pub registry: Arc<Registry>,
    pub catalog: Arc<dyn Storage>,
    pub cache: Arc<MemoryCache>,
    /// Server-scope extension settings from `selta.toml`.
    pub server_settings: Arc<HashMap<String, Value>>,
    pub stats: Arc<Stats>,
    pub semaphores: RwLock<HashMap<String, Arc<Semaphore>>>,
    pub jobs: RwLock<HashMap<Uuid, Arc<Job>>>,
    /// Extensions dialed in per pool over websocket (docs/05 §websocket).
    pub pool_hosts: RwLock<HashMap<String, PoolHosts>>,
}

pub struct Job {
    pub pool: String,
    pub report: RwLock<Option<Report>>,
    pub done: watch::Sender<bool>,
    pub handle: Mutex<Option<JoinHandle<()>>>,
    pub canceled: AtomicBool,
}

impl Job {
    pub fn new(pool: String) -> (Arc<Job>, Uuid) {
        let (done, _) = watch::channel(false);
        (
            Arc::new(Job {
                pool,
                report: RwLock::new(None),
                done,
                handle: Mutex::new(None),
                canceled: AtomicBool::new(false),
            }),
            Uuid::new_v4(),
        )
    }
}

impl AppState {
    pub async fn pool_semaphore(&self, pool: &str, max_concurrency: usize) -> Arc<Semaphore> {
        let mut map = self.semaphores.write().await;
        map.entry(pool.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(max_concurrency.max(1))))
            .clone()
    }

    /// The base registry plus the pool's own dialed-in hosts. Cloning the
    /// registry is a map of `Arc`s — cheap, per request, no locking after.
    pub async fn effective_registry(&self, pool: &str) -> Arc<Registry> {
        let hosts = self.pool_hosts.read().await;
        match hosts.get(pool) {
            Some(entries) if !entries.is_empty() => {
                let mut registry = (*self.registry).clone();
                for entry in entries.values() {
                    let host: Arc<dyn ExtensionHost> = entry.host.clone();
                    // Clashes were rejected at connect; a race with a fresh
                    // server host loses to the server host.
                    let _ = registry.register(vec![(*entry.decl).clone()], host);
                }
                Arc::new(registry)
            }
            _ => self.registry.clone(),
        }
    }

    pub async fn pool_host_extensions(&self, pool: &str) -> HashSet<String> {
        self.pool_hosts
            .read()
            .await
            .get(pool)
            .map(|entries| entries.keys().cloned().collect())
            .unwrap_or_default()
    }
}

pub const BUILTINS: &[&str] = &["one_of", "range", "regex", "len", "non_empty", "cmd"];

/// Registration-time validation for a pool: core meta-validation plus
/// per-pool enablement (docs/06). Pool-host extensions are implicitly
/// granted — dialing them in was the app's own act. A schema that registers
/// cannot fail at verify time for reasons the catalog could have caught.
pub fn pool_validate(
    schema: &Node,
    pool: &PoolConfig,
    registry: &Registry,
    pool_host_exts: &HashSet<String>,
) -> Vec<String> {
    let mut errors = meta::validate(schema, registry);
    walk_node(schema, pool, pool_host_exts, &mut errors);
    errors
}

fn walk_node(
    node: &Node,
    pool: &PoolConfig,
    pool_host_exts: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    for spec in &node.verify {
        walk_spec(spec, pool, pool_host_exts, errors);
    }
    match &node.ty {
        Type::Object { fields, .. } => {
            for field in fields.values() {
                walk_node(&field.node, pool, pool_host_exts, errors);
            }
        }
        Type::Array { item, .. } => walk_node(item, pool, pool_host_exts, errors),
        Type::Union { variants } => {
            for variant in variants {
                walk_node(variant, pool, pool_host_exts, errors);
            }
        }
        _ => {}
    }
}

fn walk_spec(
    spec: &VerifierSpec,
    pool: &PoolConfig,
    pool_host_exts: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    match spec {
        VerifierSpec::AllOf { all_of } => {
            for child in all_of {
                walk_spec(child, pool, pool_host_exts, errors);
            }
        }
        VerifierSpec::AnyOf { any_of } => {
            for child in any_of {
                walk_spec(child, pool, pool_host_exts, errors);
            }
        }
        VerifierSpec::Not { not, .. } => walk_spec(not, pool, pool_host_exts, errors),
        VerifierSpec::Leaf(leaf) => {
            if !BUILTINS.contains(&leaf.ext.as_str())
                && !pool.extensions.iter().any(|e| e == &leaf.ext)
                && !pool_host_exts.contains(&leaf.ext)
            {
                errors.push(format!(
                    "extension '{}' is not enabled for pool '{}'",
                    leaf.ext, pool.name
                ));
            }
            if leaf.ext == "cmd" {
                match leaf.config.get("name").and_then(serde_json::Value::as_str) {
                    Some(name) if pool.cmd.iter().any(|c| c == name) => {}
                    Some(name) => errors.push(format!(
                        "cmd template '{name}' is not enabled for pool '{}'",
                        pool.name
                    )),
                    None => errors.push(
                        "cmd: config.name must be a literal string — command names \
                         cannot come from $env holes"
                            .to_string(),
                    ),
                }
            }
        }
    }
}
