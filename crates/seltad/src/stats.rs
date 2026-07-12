//! Monitoring at the contract boundary (docs/06 §Monitoring): per pool ×
//! extension counters, aggregated in memory and persisted through the
//! storage trait as opaque snapshots. Agreement is the one metric only
//! Selta can compute: the quality signal for a judge.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use selta_core::{CallOutcome, Monitor, VoteTally, WireUsage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::storage::StatsRow;

const LATENCY_WINDOW: usize = 1024;

#[derive(Default)]
pub struct Stats {
    inner: Mutex<HashMap<(String, String), ExtStats>>,
    /// Set on every record, cleared by `take_dirty` — the flusher skips
    /// writes when nothing changed.
    dirty: AtomicBool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct ExtStats {
    calls: u64,
    errors: u64,
    timeouts: u64,
    latency_ms: Vec<u64>,
    input_tokens: u64,
    output_tokens: u64,
    cost_usd: f64,
    votes_pass: u64,
    votes_fail: u64,
    rounds: u64,
    agreement_sum: f64,
}

impl ExtStats {
    fn record_latency(&mut self, ms: u64) {
        if self.latency_ms.len() < LATENCY_WINDOW {
            self.latency_ms.push(ms);
        } else {
            self.latency_ms[(self.calls as usize) % LATENCY_WINDOW] = ms;
        }
    }

    fn percentile(&self, p: f64) -> Option<u64> {
        if self.latency_ms.is_empty() {
            return None;
        }
        let mut sorted = self.latency_ms.clone();
        sorted.sort_unstable();
        Some(sorted[((sorted.len() - 1) as f64 * p).round() as usize])
    }

    fn to_json(&self) -> Value {
        json!({
            "calls": self.calls,
            "errors": self.errors,
            "timeouts": self.timeouts,
            "latency_ms": { "p50": self.percentile(0.50), "p95": self.percentile(0.95) },
            "usage": {
                "input_tokens": self.input_tokens,
                "output_tokens": self.output_tokens,
                "cost_usd": self.cost_usd,
            },
            "votes": { "pass": self.votes_pass, "fail": self.votes_fail },
            "agreement": if self.rounds > 0 {
                Value::from(self.agreement_sum / self.rounds as f64)
            } else {
                Value::Null
            },
        })
    }
}

impl Stats {
    pub fn pool_json(&self, pool: &str) -> Value {
        let inner = self.inner.lock().unwrap();
        let mut keys: Vec<&(String, String)> = inner.keys().filter(|(p, _)| p == pool).collect();
        keys.sort();
        let mut out = Map::new();
        for key in keys {
            out.insert(key.1.clone(), inner[key].to_json());
        }
        Value::Object(out)
    }

    /// Every counter as storage rows, sorted for stable output.
    pub fn snapshot(&self) -> Vec<StatsRow> {
        let inner = self.inner.lock().unwrap();
        let mut keys: Vec<&(String, String)> = inner.keys().collect();
        keys.sort();
        keys.into_iter()
            .filter_map(|key| {
                Some(StatsRow {
                    pool: key.0.clone(),
                    ext: key.1.clone(),
                    data: serde_json::to_value(&inner[key]).ok()?,
                })
            })
            .collect()
    }

    /// Rehydrate counters from storage at boot; rows that no longer parse
    /// are dropped rather than taking the daemon down.
    pub fn restore(&self, rows: Vec<StatsRow>) {
        let mut inner = self.inner.lock().unwrap();
        for row in rows {
            if let Ok(entry) = serde_json::from_value::<ExtStats>(row.data) {
                inner.insert((row.pool, row.ext), entry);
            }
        }
    }

    /// True if anything was recorded since the last call.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }
}

/// The engine-facing half: one per verify job, bound to its pool.
pub struct PoolMonitor {
    pub pool: String,
    pub stats: Arc<Stats>,
}

impl Monitor for PoolMonitor {
    fn call(&self, ext: &str, outcome: CallOutcome, elapsed_ms: u64, usage: Option<&WireUsage>) {
        let mut inner = self.stats.inner.lock().unwrap();
        let entry = inner
            .entry((self.pool.clone(), ext.to_string()))
            .or_default();
        entry.record_latency(elapsed_ms);
        entry.calls += 1;
        match outcome {
            CallOutcome::Ok => {}
            CallOutcome::Error => entry.errors += 1,
            CallOutcome::Timeout => entry.timeouts += 1,
        }
        if let Some(usage) = usage {
            entry.input_tokens += usage.input_tokens;
            entry.output_tokens += usage.output_tokens;
            entry.cost_usd += usage.cost_usd;
        }
        self.stats.dirty.store(true, Ordering::Relaxed);
    }

    fn votes(&self, ext: &str, tally: &VoteTally) {
        let mut inner = self.stats.inner.lock().unwrap();
        let entry = inner
            .entry((self.pool.clone(), ext.to_string()))
            .or_default();
        entry.votes_pass += u64::from(tally.pass);
        entry.votes_fail += u64::from(tally.fail);
        let valid = tally.pass + tally.fail;
        if valid > 0 {
            entry.rounds += 1;
            entry.agreement_sum += f64::from(tally.pass.max(tally.fail)) / f64::from(valid);
        }
        self.stats.dirty.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_restore_roundtrip_preserves_counters_and_percentiles() {
        let stats = Arc::new(Stats::default());
        let monitor = PoolMonitor {
            pool: "app".to_string(),
            stats: stats.clone(),
        };
        for ms in [10, 20, 30, 40] {
            monitor.call("judge", CallOutcome::Ok, ms, None);
        }
        monitor.call("judge", CallOutcome::Error, 500, None);
        monitor.votes(
            "judge",
            &VoteTally {
                pass: 2,
                fail: 1,
                errors: 0,
                samples: 3,
            },
        );
        assert!(stats.take_dirty());
        assert!(!stats.take_dirty());

        let restored = Stats::default();
        restored.restore(stats.snapshot());
        assert_eq!(
            restored.pool_json("app"),
            stats.pool_json("app"),
            "restored counters must render identically"
        );
    }
}
