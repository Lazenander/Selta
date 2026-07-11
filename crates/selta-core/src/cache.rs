//! Verifier cache (docs/03 §Caching). Sound only for deterministic extensions
//! — a Dirac kernel is copyable (docs/08); the engine never consults it for
//! non-deterministic ones.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::host::Envelope;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: u64) -> Option<Envelope>;
    async fn put(&self, key: u64, envelope: Envelope);
}

pub struct NoCache;

#[async_trait]
impl Cache for NoCache {
    async fn get(&self, _key: u64) -> Option<Envelope> {
        None
    }

    async fn put(&self, _key: u64, _envelope: Envelope) {}
}

#[derive(Default)]
pub struct MemoryCache {
    inner: Mutex<HashMap<u64, Envelope>>,
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: u64) -> Option<Envelope> {
        self.inner.lock().unwrap().get(&key).cloned()
    }

    async fn put(&self, key: u64, envelope: Envelope) {
        self.inner.lock().unwrap().insert(key, envelope);
    }
}

/// Content hash over the extension semantics, resolved config, settings
/// fingerprint, value, call site, recursion budget, and exactly the context
/// fields the extension declared in `needs`.
/// Settings participate because a verdict from one model is not a verdict
/// from another (docs/03 §Caching).
pub struct KeyMaterial<'a> {
    pub ext: &'a str,
    pub semantic_revision: &'a str,
    pub config: &'a Value,
    pub settings_fingerprint: &'a str,
    pub value: &'a Value,
    pub path: &'a str,
    pub depth: u32,
    pub root: Option<&'a Value>,
    pub env: Option<&'a Value>,
}

pub fn key(material: KeyMaterial<'_>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    material.ext.hash(&mut hasher);
    material.semantic_revision.hash(&mut hasher);
    material.config.to_string().hash(&mut hasher);
    material.settings_fingerprint.hash(&mut hasher);
    material.value.to_string().hash(&mut hasher);
    material.path.hash(&mut hasher);
    material.depth.hash(&mut hasher);
    material.root.map(Value::to_string).hash(&mut hasher);
    material.env.map(Value::to_string).hash(&mut hasher);
    hasher.finish()
}
