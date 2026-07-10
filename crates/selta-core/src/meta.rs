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
        walk_spec(spec, path, registry, errors);
    }
    match &node.ty {
        Type::Object { fields, .. } => {
            for (name, field) in fields {
                walk_node(&field.node, &path.child_key(name), registry, errors);
            }
        }
        Type::Array { item, .. } => {
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

fn walk_spec(spec: &VerifierSpec, path: &Path, registry: &Registry, errors: &mut Vec<String>) {
    match spec {
        VerifierSpec::AllOf { all_of } => {
            if all_of.is_empty() {
                errors.push(format!("{path}: all_of must not be empty"));
            }
            for child in all_of {
                walk_spec(child, path, registry, errors);
            }
        }
        VerifierSpec::AnyOf { any_of } => {
            if any_of.is_empty() {
                errors.push(format!("{path}: any_of must not be empty"));
            }
            for child in any_of {
                walk_spec(child, path, registry, errors);
            }
        }
        VerifierSpec::Not { not, message } => {
            if message.trim().is_empty() {
                errors.push(format!(
                    "{path}: not requires a non-empty message — a passing inner check \
                     produces no delta to negate"
                ));
            }
            walk_spec(not, path, registry, errors);
        }
        VerifierSpec::Leaf(leaf) => {
            let Some(decl) = registry.decl(&leaf.ext) else {
                errors.push(format!("{path}: unknown extension '{}'", leaf.ext));
                return;
            };
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
                if let VotePolicy::AtLeast { at_least } = sampling.vote {
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
                if let Some(min_valid) = sampling.min_valid {
                    if min_valid > sampling.samples {
                        errors.push(format!(
                            "{path}: min_valid ({min_valid}) exceeds samples ({})",
                            sampling.samples
                        ));
                    }
                }
            }
            if !contains_env_ref(&leaf.config) {
                if let Some(config_schema) = &decl.config_schema {
                    if !structure_only_ok(config_schema, &leaf.config) {
                        errors.push(format!(
                            "{path}: config for '{}' does not satisfy its config_schema",
                            leaf.ext
                        ));
                    }
                }
            }
        }
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
            items.iter().all(|item_value| structure_only_ok(item, item_value))
        }
        Type::Union { variants } => variants
            .iter()
            .any(|variant| structure_only_ok(variant, value)),
    }
}
