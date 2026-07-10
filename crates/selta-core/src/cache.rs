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

/// Content hash over the extension, resolved config, settings fingerprint,
/// value, and exactly the context fields the extension declared in `needs`.
/// Settings participate because a verdict from one model is not a verdict
/// from another (docs/03 §Caching).
pub fn key(
    ext: &str,
    config: &Value,
    settings_fingerprint: &str,
    value: &Value,
    root: Option<&Value>,
    env: Option<&Value>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ext.hash(&mut hasher);
    config.to_string().hash(&mut hasher);
    settings_fingerprint.hash(&mut hasher);
    value.to_string().hash(&mut hasher);
    root.map(Value::to_string).hash(&mut hasher);
    env.map(Value::to_string).hash(&mut hasher);
    hasher.finish()
}
