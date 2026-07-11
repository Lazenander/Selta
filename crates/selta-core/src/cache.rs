//! Verifier cache (docs/03 §Caching). Sound only for deterministic extensions
//! — a Dirac kernel is copyable (docs/08); the engine never consults it for
//! non-deterministic ones.

use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::host::Envelope;

/// Default number of verifier envelopes retained by [`MemoryCache`].
pub const DEFAULT_MEMORY_CACHE_MAX_ENTRIES: usize = 16_384;
/// Default approximate serialized-envelope weight retained by [`MemoryCache`].
pub const DEFAULT_MEMORY_CACHE_MAX_BYTES: usize = 128 * 1024 * 1024;

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

pub struct MemoryCache {
    inner: Mutex<MemoryCacheInner>,
    max_entries: usize,
    max_bytes: usize,
}

struct CacheEntry {
    envelope: Envelope,
    weight: usize,
    last_used: u128,
}

#[derive(Default)]
struct MemoryCacheInner {
    entries: HashMap<u64, CacheEntry>,
    /// `(logical access time, cache key)` makes eviction independent of
    /// `HashMap` iteration order and therefore deterministic.
    recency: BTreeSet<(u128, u64)>,
    total_weight: usize,
    clock: u128,
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::with_limits(
            DEFAULT_MEMORY_CACHE_MAX_ENTRIES,
            DEFAULT_MEMORY_CACHE_MAX_BYTES,
        )
    }
}

impl MemoryCache {
    /// Construct a bounded least-recently-used cache. If either limit is zero,
    /// caching is disabled. Byte accounting is the serialized JSON length of
    /// each envelope; container overhead is deliberately approximate.
    pub fn with_limits(max_entries: usize, max_bytes: usize) -> MemoryCache {
        MemoryCache {
            inner: Mutex::new(MemoryCacheInner::default()),
            max_entries,
            max_bytes,
        }
    }

    fn enabled(&self) -> bool {
        self.max_entries > 0 && self.max_bytes > 0
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: u64) -> Option<Envelope> {
        if !self.enabled() {
            return None;
        }
        let mut inner = self.inner.lock().unwrap();
        let old_stamp = inner.entries.get(&key)?.last_used;
        inner.recency.remove(&(old_stamp, key));
        let new_stamp = next_stamp(&mut inner);
        let envelope = {
            let entry = inner
                .entries
                .get_mut(&key)
                .expect("entry exists after recency removal");
            entry.last_used = new_stamp;
            entry.envelope.clone()
        };
        inner.recency.insert((new_stamp, key));
        Some(envelope)
    }

    async fn put(&self, key: u64, envelope: Envelope) {
        if !self.enabled() {
            return;
        }
        let Some(weight) = serialized_weight(&envelope) else {
            return;
        };
        let mut inner = self.inner.lock().unwrap();
        remove_entry(&mut inner, key);
        if weight > self.max_bytes {
            return;
        }

        while inner.entries.len() >= self.max_entries
            || inner.total_weight > self.max_bytes - weight
        {
            let Some((_, oldest_key)) = inner.recency.first().copied() else {
                break;
            };
            remove_entry(&mut inner, oldest_key);
        }

        let stamp = next_stamp(&mut inner);
        inner.total_weight += weight;
        inner.entries.insert(
            key,
            CacheEntry {
                envelope,
                weight,
                last_used: stamp,
            },
        );
        inner.recency.insert((stamp, key));
    }
}

fn serialized_weight(envelope: &Envelope) -> Option<usize> {
    serde_json::to_vec(envelope).ok().map(|bytes| bytes.len())
}

fn next_stamp(inner: &mut MemoryCacheInner) -> u128 {
    if inner.clock == u128::MAX {
        let keys = inner
            .recency
            .iter()
            .map(|(_, key)| *key)
            .collect::<Vec<_>>();
        inner.recency.clear();
        inner.clock = 0;
        for key in keys {
            inner.clock += 1;
            let stamp = inner.clock;
            if let Some(entry) = inner.entries.get_mut(&key) {
                entry.last_used = stamp;
                inner.recency.insert((stamp, key));
            }
        }
    }
    inner.clock += 1;
    inner.clock
}

fn remove_entry(inner: &mut MemoryCacheInner, key: u64) {
    if let Some(entry) = inner.entries.remove(&key) {
        inner.recency.remove(&(entry.last_used, key));
        inner.total_weight = inner.total_weight.saturating_sub(entry.weight);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::WireDelta;

    #[test]
    fn default_cache_uses_the_public_bounds() {
        let cache = MemoryCache::default();
        assert_eq!(cache.max_entries, DEFAULT_MEMORY_CACHE_MAX_ENTRIES);
        assert_eq!(cache.max_bytes, DEFAULT_MEMORY_CACHE_MAX_BYTES);
    }

    #[tokio::test]
    async fn zero_limits_disable_the_cache() {
        for cache in [
            MemoryCache::with_limits(0, usize::MAX),
            MemoryCache::with_limits(usize::MAX, 0),
        ] {
            cache.put(1, Envelope::pass()).await;
            assert!(cache.get(1).await.is_none());
        }
    }

    #[tokio::test]
    async fn entry_limit_evicts_the_least_recently_used_key() {
        let cache = MemoryCache::with_limits(2, usize::MAX);
        cache.put(1, Envelope::pass()).await;
        cache.put(2, Envelope::pass()).await;
        assert!(cache.get(1).await.is_some(), "key 1 becomes newest");

        cache.put(3, Envelope::pass()).await;

        assert!(cache.get(1).await.is_some());
        assert!(cache.get(2).await.is_none(), "oldest key is evicted");
        assert!(cache.get(3).await.is_some());
    }

    #[tokio::test]
    async fn byte_limit_is_weighted_and_oversized_values_are_not_cached() {
        let small = Envelope::pass();
        let small_weight = serialized_weight(&small).expect("envelope serializes");
        let cache = MemoryCache::with_limits(10, small_weight * 2);
        cache.put(1, small.clone()).await;
        cache.put(2, small).await;
        assert!(cache.get(1).await.is_some());
        assert!(cache.get(2).await.is_some());

        let oversized = Envelope::fail(WireDelta::message("x".repeat(small_weight * 3)));
        cache.put(3, oversized).await;
        assert!(cache.get(3).await.is_none());
        assert!(cache.get(1).await.is_some());
        assert!(cache.get(2).await.is_some());
    }

    #[tokio::test]
    async fn byte_pressure_evicts_oldest_entries() {
        let envelope = Envelope::fail(WireDelta::message("x".repeat(256)));
        let weight = serialized_weight(&envelope).expect("envelope serializes");
        let cache = MemoryCache::with_limits(10, weight * 2 - 1);

        cache.put(1, envelope.clone()).await;
        cache.put(2, envelope).await;

        assert!(cache.get(1).await.is_none(), "oldest weight is evicted");
        assert!(cache.get(2).await.is_some());
    }

    #[tokio::test]
    async fn replacing_a_key_updates_weight_without_leaking_recency_entries() {
        let small = Envelope::pass();
        let replacement = Envelope::fail(WireDelta::message("replacement"));
        let limit = serialized_weight(&replacement).expect("envelope serializes");
        let cache = MemoryCache::with_limits(1, limit);

        cache.put(7, small).await;
        cache.put(7, replacement).await;

        assert!(cache.get(7).await.is_some());
        let inner = cache.inner.lock().unwrap();
        assert_eq!(inner.entries.len(), 1);
        assert_eq!(inner.recency.len(), 1);
        assert!(inner.total_weight <= limit);
    }
}
