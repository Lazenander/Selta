//! The arbiter's sole dependency seam into stable Selta.

use anyhow::{anyhow, bail, Result};
use selta_core::{
    verify, AdmissionPolicy, AdmittedNode, BuiltinAdmissionLimits, Input, Mode, NoCache, Options,
    Registry, Runtime, Verdict,
};
use serde_json::{json, Value};

use crate::json::{parse_ijson, ParseLimits};

pub(crate) struct StableSelta {
    registry: Registry,
    cache: NoCache,
    policy: AdmissionPolicy,
}

pub(crate) struct AdmittedSchema(AdmittedNode);

impl StableSelta {
    pub(crate) fn new() -> Self {
        Self {
            registry: Registry::with_pure_builtins(),
            cache: NoCache,
            policy: AdmissionPolicy::pure_only()
                .with_builtin_limits(BuiltinAdmissionLimits::new(Some(128), Some(512))),
        }
    }

    pub(crate) fn admit_schema(&self, source: &[u8]) -> Result<AdmittedSchema> {
        let neutral = parse_ijson(source, ParseLimits::SCHEMA)?;
        reject_env_references(&neutral)?;
        check_node_bounds(&neutral, "")?;
        self.registry
            .admit_source(source, &self.policy)
            .map(AdmittedSchema)
            .map_err(|issues| {
                let rendered = issues
                    .into_iter()
                    .map(|issue| {
                        format!(
                            "{} at {}: {}",
                            issue.code.as_str(),
                            issue.pointer,
                            issue.detail
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                anyhow!("stable Selta schema admission failed: {rendered}")
            })
    }

    pub(crate) async fn verify_value(&self, schema: &AdmittedSchema, value: &Value) -> Result<()> {
        let runtime = Runtime::new(&self.registry, &self.cache);
        let options = Options {
            mode: Mode::Strict,
            fail_fast: false,
            max_depth: 1,
            max_samples: 1,
            deadline_ms: None,
            schema_name: None,
        };
        let report = verify(
            schema.0.as_node(),
            Input::Value(value.clone()),
            &json!({}),
            &options,
            &runtime,
        )
        .await;
        if report.verdict == Verdict::Pass && report.errors.is_empty() {
            Ok(())
        } else {
            bail!(
                "stable Selta value verification failed with verdict {:?}, {} delta(s), and {} error(s)",
                report.verdict,
                report.deltas.len(),
                report.errors.len()
            )
        }
    }
}

fn reject_env_references(value: &Value) -> Result<()> {
    match value {
        Value::Array(values) => {
            for value in values {
                reject_env_references(value)?;
            }
        }
        Value::Object(values) => {
            if values.len() == 1 && values.get("$env").is_some_and(|value| value.is_string()) {
                bail!("candidate schema sources must not contain a $env reference");
            }
            for value in values.values() {
                reject_env_references(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_node_bounds(value: &Value, pointer: &str) -> Result<()> {
    let Some(node) = value.as_object() else {
        return Ok(());
    };
    if let Some(bounds) = node.get("len").and_then(Value::as_object) {
        for name in ["min", "max"] {
            if bounds
                .get(name)
                .and_then(Value::as_u64)
                .is_some_and(|bound| bound > u32::MAX as u64)
            {
                bail!("schema bound at {pointer}/len/{name} exceeds 4,294,967,295");
            }
        }
    }

    match node.get("type").and_then(Value::as_str) {
        Some("object") => {
            if let Some(fields) = node.get("fields").and_then(Value::as_object) {
                for (name, field) in fields {
                    check_node_bounds(field, &format!("{pointer}/fields/{name}"))?;
                }
            }
        }
        Some("array") => {
            if let Some(item) = node.get("item") {
                check_node_bounds(item, &format!("{pointer}/item"))?;
            }
        }
        Some("union") => {
            if let Some(variants) = node.get("variants").and_then(Value::as_array) {
                for (index, variant) in variants.iter().enumerate() {
                    check_node_bounds(variant, &format!("{pointer}/variants/{index}"))?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_profile_admits_and_verifies_a_closed_schema() {
        let stable = StableSelta::new();
        let schema = stable
            .admit_schema(
                br#"{"type":"object","fields":{"x":{"type":"int","verify":[{"ext":"range","config":{"min":0,"max":2}}]}}}"#,
            )
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime
            .block_on(stable.verify_value(&schema, &json!({"x": 2})))
            .unwrap();
        assert!(runtime
            .block_on(stable.verify_value(&schema, &json!({"x": 3})))
            .is_err());
    }

    #[test]
    fn exact_profile_rejects_process_io() {
        let stable = StableSelta::new();
        let result = stable
            .admit_schema(br#"{"type":"str","verify":[{"ext":"cmd","config":{"template":"x"}}]}"#);
        assert!(result.is_err());
    }

    #[test]
    fn candidate_preflight_rejects_env_and_host_width_bounds() {
        let stable = StableSelta::new();
        assert!(stable
            .admit_schema(
                br#"{"type":"str","verify":[{"ext":"regex","config":{"pattern":{"$env":"x"}}}]}"#,
            )
            .is_err());
        assert!(stable
            .admit_schema(br#"{"type":"array","len":{"max":4294967296},"item":{"type":"any"}}"#,)
            .is_err());
    }
}
