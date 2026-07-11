//! Shared server state: registry, catalog, settings, stats, per-pool
//! semaphores, jobs, dialed-in pool hosts — and the pool-level registration
//! checks (enablement, literal cmd names).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use selta_core::{
    meta, Determinism, ExtensionHost, MemoryCache, Node, Registry, Report, Sampling, Type,
    VerifierSpec,
};
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
                    // Every entry passed this exact atomic Registry gate while
                    // the pool-host map was write-locked at connect time.
                    registry
                        .register(vec![(*entry.decl).clone()], host)
                        .expect("validated pool-host entry remains registrable");
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
    walk_node(schema, pool, registry, pool_host_exts, &mut errors);
    errors
}

fn walk_node(
    node: &Node,
    pool: &PoolConfig,
    registry: &Registry,
    pool_host_exts: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    for spec in &node.verify {
        walk_spec(spec, pool, registry, pool_host_exts, errors);
    }
    match &node.ty {
        Type::Object { fields, .. } => {
            for field in fields.values() {
                walk_node(&field.node, pool, registry, pool_host_exts, errors);
            }
        }
        Type::Array { item, .. } => walk_node(item, pool, registry, pool_host_exts, errors),
        Type::Union { variants } => {
            for variant in variants {
                walk_node(variant, pool, registry, pool_host_exts, errors);
            }
        }
        _ => {}
    }
}

fn walk_spec(
    spec: &VerifierSpec,
    pool: &PoolConfig,
    registry: &Registry,
    pool_host_exts: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    match spec {
        VerifierSpec::AllOf { all_of } => {
            for child in all_of {
                walk_spec(child, pool, registry, pool_host_exts, errors);
            }
        }
        VerifierSpec::AnyOf { any_of } => {
            for child in any_of {
                walk_spec(child, pool, registry, pool_host_exts, errors);
            }
        }
        VerifierSpec::Not { not, .. } => walk_spec(not, pool, registry, pool_host_exts, errors),
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

            // A non-deterministic leaf always samples, even when its schema
            // omits `sampling` and therefore selects Selta's defaults. Reject
            // a version that cannot fit one leaf invocation in this pool;
            // the request-wide counter remains authoritative across sibling
            // leaves, arrays, and retry attempts.
            if registry
                .decl(&leaf.ext)
                .is_some_and(|decl| decl.determinism == Determinism::Nondeterministic)
            {
                check_sampling_budget(&leaf.ext, leaf.sampling, pool, errors);
            }
        }
    }
}

fn check_sampling_budget(
    extension: &str,
    requested: Option<Sampling>,
    pool: &PoolConfig,
    errors: &mut Vec<String>,
) {
    let sampling = requested.unwrap_or_default();
    if sampling.samples > pool.budget.max_samples_per_request {
        errors.push(format!(
            "extension '{extension}': sampling.samples ({}) exceeds pool '{}' \
             max_samples_per_request ({})",
            sampling.samples, pool.name, pool.budget.max_samples_per_request
        ));
    }
    if sampling.depth > pool.budget.max_depth {
        errors.push(format!(
            "extension '{extension}': sampling.depth ({}) exceeds pool '{}' max_depth ({})",
            sampling.depth, pool.name, pool.budget.max_depth
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use selta_core::VotePolicy;

    use super::*;
    use crate::storage::PoolBudget;

    fn pool(max_depth: u32, max_samples_per_request: u32) -> PoolConfig {
        PoolConfig {
            name: "test-pool".to_string(),
            extensions: Vec::new(),
            cmd: Vec::new(),
            settings: BTreeMap::new(),
            budget: PoolBudget {
                max_depth,
                max_samples_per_request,
                max_concurrency: 1,
            },
        }
    }

    #[test]
    fn omitted_sampling_is_checked_as_the_runtime_default() {
        let mut errors = Vec::new();
        check_sampling_budget("judge", None, &pool(0, 2), &mut errors);

        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("sampling.samples (3)"));
        assert!(errors[1].contains("sampling.depth (1)"));
    }

    #[test]
    fn explicit_sampling_must_fit_each_pool_cap() {
        let request = Sampling {
            samples: 8,
            vote: VotePolicy::default(),
            depth: 4,
            min_valid: None,
        };
        let mut errors = Vec::new();
        check_sampling_budget("judge", Some(request), &pool(3, 7), &mut errors);

        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("max_samples_per_request (7)"));
        assert!(errors[1].contains("max_depth (3)"));
    }

    #[test]
    fn feasible_sampling_is_accepted() {
        let mut errors = Vec::new();
        check_sampling_budget(
            "judge",
            Some(Sampling {
                samples: 7,
                vote: VotePolicy::default(),
                depth: 3,
                min_valid: None,
            }),
            &pool(3, 7),
            &mut errors,
        );

        assert!(errors.is_empty());
    }
}
