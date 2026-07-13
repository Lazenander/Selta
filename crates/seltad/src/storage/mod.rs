//! The persisted catalog (docs/06) behind a storage trait: pools, immutable
//! schema versions, and monitoring counters. Two backends — sqlite (default)
//! and plain files (diffable, debuggable) — one contract, chosen in
//! `selta.toml`.

mod files;
mod sqlite;

pub use files::FileStorage;
pub use sqlite::SqliteStorage;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub name: String,
    /// Host extensions this pool may use; builtins are always available,
    /// and pool-host extensions are implicitly granted by dialing in.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Command templates this pool may reference from `cmd` verifiers.
    #[serde(default)]
    pub cmd: Vec<String>,
    /// Pool-scope settings overrides, keyed by extension (docs/06).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, Value>,
    #[serde(default)]
    pub budget: PoolBudget,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PoolBudget {
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_max_samples")]
    pub max_samples_per_request: u32,
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
}

impl Default for PoolBudget {
    fn default() -> Self {
        PoolBudget {
            max_depth: default_max_depth(),
            max_samples_per_request: default_max_samples(),
            max_concurrency: default_max_concurrency(),
        }
    }
}

fn default_max_depth() -> u32 {
    2
}

fn default_max_samples() -> u32 {
    16
}

fn default_max_concurrency() -> usize {
    8
}

/// The storage contract (docs/06 §Catalog). Semantics are backend-independent:
/// pools are created once and updated whole, schema versions are immutable
/// and assigned sequentially from 1, names are `[A-Za-z0-9_-]+`.
pub trait Storage: Send + Sync {
    /// Returns false if the pool already exists.
    fn create_pool(&self, config: &PoolConfig) -> Result<bool>;
    /// Rewrite an existing pool's config — settings updates go through here;
    /// schema versions never do.
    fn update_pool(&self, config: &PoolConfig) -> Result<()>;
    fn load_pool(&self, name: &str) -> Result<Option<PoolConfig>>;
    fn list_pools(&self) -> Result<Vec<String>>;
    /// Registering again writes the next version; nothing is ever mutated.
    /// Schema sources stay as UTF-8 bytes across this boundary so strict
    /// admission can still detect duplicate object keys after a reload.
    fn register_schema(&self, pool: &str, name: &str, source: &[u8]) -> Result<u32>;
    fn load_schema(&self, pool: &str, name: &str, version: Option<u32>) -> Result<(u32, Vec<u8>)>;
    fn list_schemas(&self, pool: &str) -> Result<BTreeMap<String, Vec<u32>>>;
    /// Monitoring counters (docs/06 §Monitoring), persisted as opaque
    /// snapshots per pool × extension — storage never reads inside `data`.
    fn load_stats(&self) -> Result<Vec<StatsRow>>;
    fn save_stats(&self, rows: &[StatsRow]) -> Result<()>;
}

/// One pool × extension counter snapshot; `data` is the serialized form of
/// the in-memory counters, owned by `stats.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsRow {
    pub pool: String,
    pub ext: String,
    pub data: Value,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Which backend persists the catalog (`storage` in `selta.toml`).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    #[default]
    Sqlite,
    Files,
}

impl StorageBackend {
    /// Reject a configured backend before the daemon opens any effectful
    /// resources. Unsupported platforms never receive a weaker fallback.
    pub fn ensure_supported(self) -> Result<()> {
        #[cfg(windows)]
        if matches!(self, Self::Files) {
            anyhow::bail!("the file catalog is not supported on Windows; use storage = \"sqlite\"");
        }
        Ok(())
    }
}

/// Open the configured backend under `data`. Where FileStorage is supported, a
/// fresh sqlite catalog imports an existing file catalog at the same path once.
/// Windows rejects that migration before creating the database.
pub fn open(backend: StorageBackend, data: &Path) -> Result<Arc<dyn Storage>> {
    backend.ensure_supported()?;
    match backend {
        StorageBackend::Files => Ok(Arc::new(FileStorage::open(data.to_path_buf())?)),
        StorageBackend::Sqlite => {
            let db = data.join("selta.db");
            let fresh = !db.exists();
            #[cfg(windows)]
            if fresh && data.join("pools").is_dir() {
                anyhow::bail!(
                    "automatic import of a file catalog is not supported on Windows; \
                     migrate it on a supported platform before opening SQLite"
                );
            }
            let sqlite = SqliteStorage::open(&db)?;
            #[cfg(not(windows))]
            if fresh && data.join("pools").is_dir() {
                let files = FileStorage::open(data.to_path_buf())?;
                let imported = copy(&files, &sqlite)?;
                if imported > 0 {
                    eprintln!(
                        "seltad: imported {imported} pool(s) from the file catalog into {}",
                        db.display()
                    );
                }
            }
            Ok(Arc::new(sqlite))
        }
    }
}

/// Copy everything one backend persists into another — the migration path
/// between backends. Returns the number of pools copied.
pub fn copy(from: &dyn Storage, to: &dyn Storage) -> Result<usize> {
    let pools = from.list_pools()?;
    for name in &pools {
        let Some(config) = from.load_pool(name)? else {
            continue;
        };
        to.create_pool(&config)?;
        for (schema, versions) in from.list_schemas(name)? {
            for version in versions {
                let (_, body) = from.load_schema(name, &schema, Some(version))?;
                to.register_schema(name, &schema, &body)?;
            }
        }
    }
    to.save_stats(&from.load_stats()?)?;
    Ok(pools.len())
}
