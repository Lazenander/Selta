//! The arbiter's sole dependency seam into stable Selta.

use anyhow::{anyhow, bail, Context, Result};
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

impl AdmittedSchema {
    /// Project this already-admitted schema for candidate schema-source
    /// identity. The stable `Node` remains behind this boundary.
    #[allow(dead_code)] // Consumed by the immediately following manifest-identity slice.
    pub(crate) fn schema_source_projection(&self) -> Result<Value> {
        serde_json::to_value(self.0.as_node())
            .context("cannot project admitted Selta schema for schema-source identity")
    }
}

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
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::digest::h;
    use crate::path::{RepoPath, Repository};

    const SCHEMA_SOURCE_TAG: &str = "selta.evidence.schema-source/candidate-1";

    fn grammar_admitted(source: &[u8]) -> AdmittedSchema {
        AdmittedSchema(
            AdmittedNode::admit_source(source)
                .unwrap_or_else(|issues| panic!("schema grammar admission failed: {issues:?}")),
        )
    }

    fn repository() -> Repository {
        let root =
            fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..")).unwrap();
        Repository::open(&root).unwrap()
    }

    fn read_retained_schema(repository: &Repository, path: &str) -> Vec<u8> {
        repository
            .read_regular_file(&path.parse::<RepoPath>().unwrap(), 1_048_576)
            .unwrap()
    }

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

    #[test]
    fn schema_source_projection_expands_only_admitted_defaults() {
        let stable = StableSelta::new();
        let omitted = stable
            .admit_schema(
                br#"{"type":"object","fields":{"value":{"type":"array","item":{"type":"str","verify":[{"ext":"non_empty"}]}}}}"#,
            )
            .unwrap();
        let explicit = stable
            .admit_schema(
                br#"{"type":"object","open":false,"fields":{"value":{"type":"array","item":{"type":"str","verify":[{"ext":"non_empty","config":{}}]},"required":true}}}"#,
            )
            .unwrap();

        let expected = json!({
            "type": "object",
            "fields": {
                "value": {
                    "type": "array",
                    "item": {
                        "type": "str",
                        "verify": [{"ext": "non_empty", "config": {}}]
                    },
                    "required": true
                }
            },
            "open": false
        });
        assert_eq!(omitted.schema_source_projection().unwrap(), expected);
        assert_eq!(explicit.schema_source_projection().unwrap(), expected);
    }

    #[test]
    fn schema_source_projection_distinguishes_absent_and_supplied_sampling() {
        // Candidate 1's exact pure registry correctly rejects sampling on its
        // deterministic builtins. Core grammar admission is used here only to
        // freeze the projection of the stable Selta shape for future profiles.
        let stable = StableSelta::new();
        let absent_source = br#"{"type":"str","verify":[{"ext":"non_empty"}]}"#;
        let supplied_source = br#"{"type":"str","verify":[{"ext":"non_empty","sampling":{}}]}"#;
        assert!(stable.admit_schema(absent_source).is_ok());
        assert!(stable.admit_schema(supplied_source).is_err());
        let absent = grammar_admitted(absent_source);
        let supplied = grammar_admitted(supplied_source);

        let absent = absent.schema_source_projection().unwrap();
        let supplied = supplied.schema_source_projection().unwrap();
        assert!(absent.pointer("/verify/0/sampling").is_none());
        assert_eq!(
            supplied.pointer("/verify/0/sampling"),
            Some(&json!({"samples": 3, "vote": "majority", "depth": 1}))
        );
        assert_ne!(absent, supplied);
    }

    #[test]
    fn schema_source_projection_preserves_every_normative_optional_shape() {
        // This shape deliberately uses grammar-only extension names. It locks
        // the stable projection components that candidate 1 does not exercise
        // through its deterministic builtin profile.
        let schema = grammar_admitted(
            br#"{
                "type":"object",
                "description":"root",
                "open":true,
                "fields":{
                    "items":{
                        "type":"array",
                        "len":{"min":1},
                        "item":{
                            "type":"str",
                            "description":"leaf",
                            "verify":[{
                                "all_of":[
                                    {
                                        "ext":"first",
                                        "config":{"z":1,"a":2},
                                        "sampling":{
                                            "samples":5,
                                            "vote":{"at_least":3},
                                            "depth":2,
                                            "min_valid":4
                                        }
                                    },
                                    {
                                        "any_of":[
                                            {"ext":"second"},
                                            {"not":{"ext":"third"},"message":"not-third"}
                                        ]
                                    }
                                ]
                            }]
                        },
                        "required":false
                    }
                }
            }"#,
        );

        assert_eq!(
            schema.schema_source_projection().unwrap(),
            json!({
                "type": "object",
                "description": "root",
                "fields": {
                    "items": {
                        "type": "array",
                        "len": {"min": 1},
                        "item": {
                            "type": "str",
                            "description": "leaf",
                            "verify": [{
                                "all_of": [
                                    {
                                        "ext": "first",
                                        "config": {"z": 1, "a": 2},
                                        "sampling": {
                                            "samples": 5,
                                            "vote": {"at_least": 3},
                                            "depth": 2,
                                            "min_valid": 4
                                        }
                                    },
                                    {
                                        "any_of": [
                                            {"ext": "second", "config": {}},
                                            {
                                                "not": {"ext": "third", "config": {}},
                                                "message": "not-third"
                                            }
                                        ]
                                    }
                                ]
                            }]
                        },
                        "required": false
                    }
                },
                "open": true
            })
        );
    }

    #[test]
    fn schema_source_projection_is_neutral_to_object_member_order() {
        let stable = StableSelta::new();
        let left = stable
            .admit_schema(
                br#"{"type":"object","open":true,"fields":{"b":{"type":"int"},"a":{"type":"str"}}}"#,
            )
            .unwrap()
            .schema_source_projection()
            .unwrap();
        let right = stable
            .admit_schema(
                br#"{"fields":{"a":{"type":"str"},"b":{"type":"int"}},"open":true,"type":"object"}"#,
            )
            .unwrap()
            .schema_source_projection()
            .unwrap();

        assert_eq!(left, right);
        assert_eq!(
            h(SCHEMA_SOURCE_TAG, &left).unwrap(),
            h(SCHEMA_SOURCE_TAG, &right).unwrap()
        );
    }

    #[test]
    fn schema_source_projection_preserves_verifier_and_union_order() {
        let original = grammar_admitted(
            br#"{"type":"union","variants":[{"type":"str","verify":[{"ext":"first"},{"ext":"second"}]},{"type":"int"}]}"#,
        )
        .schema_source_projection()
        .unwrap();
        let union_swapped = grammar_admitted(
            br#"{"type":"union","variants":[{"type":"int"},{"type":"str","verify":[{"ext":"first"},{"ext":"second"}]}]}"#,
        )
        .schema_source_projection()
        .unwrap();
        let verifiers_swapped = grammar_admitted(
            br#"{"type":"union","variants":[{"type":"str","verify":[{"ext":"second"},{"ext":"first"}]},{"type":"int"}]}"#,
        )
        .schema_source_projection()
        .unwrap();

        assert_eq!(original.pointer("/variants/0/type"), Some(&json!("str")));
        assert_eq!(
            original.pointer("/variants/0/verify/0/ext"),
            Some(&json!("first"))
        );
        assert_eq!(
            original.pointer("/variants/0/verify/1/ext"),
            Some(&json!("second"))
        );
        assert_ne!(original, union_swapped);
        assert_ne!(original, verifiers_swapped);
        assert_ne!(
            h(SCHEMA_SOURCE_TAG, &original).unwrap(),
            h(SCHEMA_SOURCE_TAG, &union_swapped).unwrap()
        );
        assert_ne!(
            h(SCHEMA_SOURCE_TAG, &original).unwrap(),
            h(SCHEMA_SOURCE_TAG, &verifiers_swapped).unwrap()
        );
    }

    #[test]
    fn retained_schema_source_projections_match_published_anchors() {
        let repository = repository();
        let stable = StableSelta::new();
        let cases = [
            (
                "profiles/evidence/presence-candidate-1/schemas/unit.schema.json",
                "sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4",
            ),
            (
                "profiles/evidence/presence-candidate-1/builtin-config-schemas/non_empty.schema.json",
                "sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4",
            ),
            (
                "profiles/evidence/presence-candidate-1/schemas/text.schema.json",
                "sha256:13552c5c23c91e20e8f8a526250eed1da10df6477f799842a84b30c07921100a",
            ),
            (
                "profiles/evidence/presence-candidate-1/builtin-config-schemas/range.schema.json",
                "sha256:7e4a048ab346b9bb188e72c9257f9b2089efe875558388c775eec77c4964a3f1",
            ),
        ];

        for (path, expected) in cases {
            let source = read_retained_schema(&repository, path);
            let projection = stable
                .admit_schema(&source)
                .unwrap()
                .schema_source_projection()
                .unwrap();
            assert_eq!(
                h(SCHEMA_SOURCE_TAG, &projection).unwrap().to_string(),
                expected
            );
        }
    }
}
