//! Extension settings (docs/05 §Three kinds of configuration): how an
//! extension operates — model, endpoint, key. Values and their resolution
//! (server ⊕ pool, secrets) live with the caller; the engine only asks for
//! the outcome per extension and records the fingerprint for attribution.

use serde_json::Value;

/// Resolved settings plus a stable fingerprint. The fingerprint is computed
/// with secret values excluded (the caller hashes the redacted form) and is
/// what reports and cache keys carry — a verdict from one model is not a
/// verdict from another (docs/03 §Caching).
#[derive(Debug, Clone)]
pub struct ResolvedSettings {
    pub value: Value,
    pub fingerprint: String,
}

impl ResolvedSettings {
    pub fn empty() -> ResolvedSettings {
        let value = Value::Object(serde_json::Map::new());
        let fingerprint = fingerprint(&value);
        ResolvedSettings { value, fingerprint }
    }
}

pub trait SettingsResolver: Send + Sync {
    /// Settings for one extension. `Err` makes the check inconclusive —
    /// a misconfigured verifier is "Selta could not find out", never a delta.
    fn resolve(&self, ext: &str) -> Result<ResolvedSettings, String>;
}

/// No settings anywhere — the embedded default.
pub struct NoSettings;

impl SettingsResolver for NoSettings {
    fn resolve(&self, _ext: &str) -> Result<ResolvedSettings, String> {
        Ok(ResolvedSettings::empty())
    }
}

/// FNV-1a over the canonical JSON text — stable across runs and platforms,
/// which `DefaultHasher` does not guarantee.
pub fn fingerprint(settings: &Value) -> String {
    let text = settings.to_string();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a:{hash:016x}")
}
