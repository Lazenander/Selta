//! Meta-validation (docs/02 §Meta-validation): a schema that registers cannot
//! fail at verify time for reasons the catalog could have caught.

use serde_json::Value;

use crate::host::Determinism;
use crate::path::Path;
use crate::registry::Registry;
use crate::schema::{Node, Type, VerifierSpec, VotePolicy};

pub fn validate(schema: &Node, registry: &Registry) -> Vec<String> {
    let mut errors = Vec::new();
    walk_node(schema, &Path::root(), registry, &mut errors);
    errors
}

fn walk_node(node: &Node, path: &Path, registry: &Registry, errors: &mut Vec<String>) {
    for spec in &node.verify {
        let _ = walk_spec(spec, &node.ty, path, registry, errors);
    }
    match &node.ty {
        Type::Object { fields, .. } => {
            for (name, field) in fields {
                walk_node(&field.node, &path.child_key(name), registry, errors);
            }
        }
        Type::Array { item, len } => {
            if let Some(bounds) = len {
                if matches!((bounds.min, bounds.max), (Some(min), Some(max)) if min > max) {
                    errors.push(format!("{path}: array len.min must not exceed len.max"));
                }
            }
            walk_node(item, &path.child_index(0), registry, errors);
        }
        Type::Union { variants } => {
            if variants.is_empty() {
                errors.push(format!("{path}: union must have at least one variant"));
            }
            for variant in variants {
                walk_node(variant, path, registry, errors);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceUse {
    Legacy,
    Cautious,
    Invalid,
}

fn walk_spec(
    spec: &VerifierSpec,
    node_type: &Type,
    path: &Path,
    registry: &Registry,
    errors: &mut Vec<String>,
) -> EvidenceUse {
    match spec {
        VerifierSpec::AllOf { all_of } => {
            if all_of.is_empty() {
                errors.push(format!("{path}: all_of must not be empty"));
            }
            let modes = all_of
                .iter()
                .map(|child| walk_spec(child, node_type, path, registry, errors))
                .collect::<Vec<_>>();
            combine_evidence_modes(&modes, path, errors)
        }
        VerifierSpec::AnyOf { any_of } => {
            if any_of.is_empty() {
                errors.push(format!("{path}: any_of must not be empty"));
            }
            let modes = any_of
                .iter()
                .map(|child| walk_spec(child, node_type, path, registry, errors))
                .collect::<Vec<_>>();
            combine_evidence_modes(&modes, path, errors)
        }
        VerifierSpec::Not { not, message } => {
            if message.trim().is_empty() {
                errors.push(format!(
                    "{path}: not requires a non-empty message — a passing inner check \
                     produces no delta to negate"
                ));
            }
            walk_spec(not, node_type, path, registry, errors)
        }
        VerifierSpec::Leaf(leaf) => {
            let evidence = if leaf.evidence.is_some() {
                EvidenceUse::Cautious
            } else {
                EvidenceUse::Legacy
            };
            let Some(decl) = registry.decl(&leaf.ext) else {
                errors.push(format!("{path}: unknown extension '{}'", leaf.ext));
                return evidence;
            };
            if !decl.accepted_input.accepts_type(node_type) {
                errors.push(format!(
                    "{path}: extension '{}' does not accept node type '{}'",
                    leaf.ext,
                    node_type.name()
                ));
            }
            if let Some(sampling) = &leaf.sampling {
                if decl.determinism == Determinism::Deterministic {
                    errors.push(format!(
                        "{path}: sampling on deterministic extension '{}'",
                        leaf.ext
                    ));
                }
                if sampling.samples == 0 {
                    errors.push(format!("{path}: sampling.samples must be at least 1"));
                }
                if leaf.evidence.is_none() {
                    if let VotePolicy::AtLeast { at_least } = sampling.vote {
                        if at_least == 0 {
                            errors.push(format!("{path}: at_least must be at least 1"));
                        }
                        if at_least > sampling.samples {
                            errors.push(format!(
                                "{path}: at_least ({at_least}) exceeds samples ({})",
                                sampling.samples
                            ));
                        }
                    }
                    if let VotePolicy::Ratio { ratio } = sampling.vote {
                        if !(ratio > 0.0 && ratio <= 1.0) {
                            errors.push(format!("{path}: ratio must be in (0, 1], got {ratio}"));
                        }
                    }
                }
                if let Some(min_valid) = sampling.min_valid {
                    if min_valid == 0 {
                        errors.push(format!("{path}: min_valid must be at least 1"));
                    }
                    if min_valid > sampling.samples {
                        errors.push(format!(
                            "{path}: min_valid ({min_valid}) exceeds samples ({})",
                            sampling.samples
                        ));
                    }
                }
            }
            if !contains_env_ref(&leaf.config) {
                let structure_valid = decl
                    .config_schema
                    .as_ref()
                    .is_none_or(|config_schema| structure_only_ok(config_schema, &leaf.config));
                if !structure_valid {
                    errors.push(format!(
                        "{path}: config for '{}' does not satisfy its config_schema",
                        leaf.ext
                    ));
                } else if let Err(message) = decl.preflight_config(&leaf.config) {
                    errors.push(format!(
                        "{path}: config for '{}' failed semantic preflight: {message}",
                        leaf.ext
                    ));
                }
            }
            evidence
        }
    }
}

fn combine_evidence_modes(
    modes: &[EvidenceUse],
    path: &Path,
    errors: &mut Vec<String>,
) -> EvidenceUse {
    let legacy = modes.contains(&EvidenceUse::Legacy);
    let cautious = modes.contains(&EvidenceUse::Cautious);
    if legacy && cautious {
        errors.push(format!(
            "{path}: a verifier combinator subtree must be wholly legacy or wholly cautious"
        ));
        EvidenceUse::Invalid
    } else if modes.contains(&EvidenceUse::Invalid) {
        EvidenceUse::Invalid
    } else if cautious {
        EvidenceUse::Cautious
    } else {
        EvidenceUse::Legacy
    }
}

/// True if any part of the config is a `{ "$env": "path" }` hole (docs/02).
pub(crate) fn contains_env_ref(config: &Value) -> bool {
    match config {
        Value::Object(map) => {
            if map.len() == 1 && map.contains_key("$env") {
                return true;
            }
            map.values().any(contains_env_ref)
        }
        Value::Array(items) => items.iter().any(contains_env_ref),
        _ => false,
    }
}

/// Structure-only check: types, required fields, closed objects — no verifier
/// execution, strict numerics. Used for config and settings validation, where
/// running verifiers would regress infinitely.
pub fn structure_only_ok(node: &Node, value: &Value) -> bool {
    match &node.ty {
        Type::Null => value.is_null(),
        Type::Bool => value.is_boolean(),
        Type::Int => value.as_i64().is_some() || value.as_u64().is_some(),
        Type::Float => value.is_number(),
        Type::Str => value.is_string(),
        Type::Any => true,
        Type::Object { fields, open } => {
            let Value::Object(map) = value else {
                return false;
            };
            for (name, field) in fields {
                match map.get(name) {
                    Some(child) => {
                        if !structure_only_ok(&field.node, child) {
                            return false;
                        }
                    }
                    None => {
                        if field.required {
                            return false;
                        }
                    }
                }
            }
            *open || map.keys().all(|k| fields.contains_key(k))
        }
        Type::Array { item, len } => {
            let Value::Array(items) = value else {
                return false;
            };
            if let Some(bounds) = len {
                if bounds.min.map(|lo| items.len() < lo).unwrap_or(false)
                    || bounds.max.map(|hi| items.len() > hi).unwrap_or(false)
                {
                    return false;
                }
            }
            items
                .iter()
                .all(|item_value| structure_only_ok(item, item_value))
        }
        Type::Union { variants } => variants
            .iter()
            .any(|variant| structure_only_ok(variant, value)),
    }
}
