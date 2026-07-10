//! Settings resolution (docs/05 §Three kinds of configuration, docs/06
//! §Configuration resolution): merge server ⊕ pool with the pool winning,
//! fingerprint over the redacted form, then inject secrets from the daemon's
//! environment — never stored, never echoed.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use selta_core::{ResolvedSettings, SettingsResolver};
use serde_json::{Map, Value};

/// One pool's view of extension settings, handed to the engine per job.
pub struct PoolSettings {
    pub server: Arc<HashMap<String, Value>>,
    pub pool: BTreeMap<String, Value>,
}

impl SettingsResolver for PoolSettings {
    fn resolve(&self, ext: &str) -> Result<ResolvedSettings, String> {
        let merged = merge(self.server.get(ext), self.pool.get(ext));
        let fingerprint = selta_core::settings::fingerprint(&redact(&merged));
        let value = inject_secrets(&merged)?;
        Ok(ResolvedSettings { value, fingerprint })
    }
}

/// Shallow object merge, pool key wins; a non-object pool value replaces
/// outright.
pub fn merge(server: Option<&Value>, pool: Option<&Value>) -> Value {
    match (server, pool) {
        (Some(Value::Object(server)), Some(Value::Object(pool))) => {
            let mut out = server.clone();
            for (key, value) in pool {
                out.insert(key.clone(), value.clone());
            }
            Value::Object(out)
        }
        (_, Some(pool)) => pool.clone(),
        (Some(server), None) => server.clone(),
        (None, None) => Value::Object(Map::new()),
    }
}

fn secret_name(value: &Value) -> Option<&str> {
    let map = value.as_object()?;
    if map.len() == 1 {
        map.get("$secret")?.as_str()
    } else {
        None
    }
}

/// Secret values excluded, references kept — the form fingerprints are
/// computed over and API responses show.
pub fn redact(settings: &Value) -> Value {
    if let Some(name) = secret_name(settings) {
        return Value::String(format!("$secret:{name}"));
    }
    match settings {
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), redact(v))).collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        other => other.clone(),
    }
}

/// `{ "$secret": "NAME" }` resolves from the daemon's environment at call
/// time. A missing secret makes the check inconclusive, never a delta.
pub fn inject_secrets(settings: &Value) -> Result<Value, String> {
    if let Some(name) = secret_name(settings) {
        return std::env::var(name)
            .map(Value::String)
            .map_err(|_| format!("secret '{name}' is not set in the daemon environment"));
    }
    match settings {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, value) in map {
                out.insert(key.clone(), inject_secrets(value)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => Ok(Value::Array(
            items.iter().map(inject_secrets).collect::<Result<_, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

/// The redacted merged settings and their fingerprint — what discovery and
/// settings endpoints return.
pub fn public_view(
    server: &HashMap<String, Value>,
    pool: &BTreeMap<String, Value>,
    ext: &str,
) -> (Value, String) {
    let redacted = redact(&merge(server.get(ext), pool.get(ext)));
    let fingerprint = selta_core::settings::fingerprint(&redacted);
    (redacted, fingerprint)
}
