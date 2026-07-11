//! Registry-aware admission layered on the closed raw schema grammar.
//!
//! Raw admission proves that a source is Selta. This layer proves that every
//! verifier in that source is meaningful for one concrete registry and one
//! explicit effect/resource policy.

use std::collections::HashSet;

use serde_json::Value;

use crate::host::{Determinism, EffectClass};
use crate::meta::contains_env_ref;
use crate::registry::Registry;
use crate::schema::{Node, Type, VerifierSpec};
use crate::strict_admission::{
    child_pointer, validate_config_structure_with_env_holes, AdmittedNode, MetaIssue, MetaIssueCode,
};

/// Optional resource ceilings for Selta's in-process builtin configuration.
///
/// These are consumer policy, not builtin correctness rules. For example, a
/// proof-oriented consumer can bound an otherwise valid `one_of` list without
/// Selta embedding that consumer's profile or product name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuiltinAdmissionLimits {
    pub max_one_of_values: Option<usize>,
    pub max_regex_pattern_bytes: Option<usize>,
}

impl BuiltinAdmissionLimits {
    pub const fn new(
        max_one_of_values: Option<usize>,
        max_regex_pattern_bytes: Option<usize>,
    ) -> Self {
        Self {
            max_one_of_values,
            max_regex_pattern_bytes,
        }
    }
}

/// Registry-aware policy for one admission boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPolicy {
    allowed_effect_classes: Vec<EffectClass>,
    builtin_limits: BuiltinAdmissionLimits,
}

impl AdmissionPolicy {
    /// Construct a policy from the exact effect classes this boundary grants.
    /// An empty iterator intentionally denies every verifier extension.
    pub fn new(allowed_effect_classes: impl IntoIterator<Item = EffectClass>) -> Self {
        let requested = allowed_effect_classes.into_iter().collect::<Vec<_>>();
        let allowed = [
            EffectClass::Pure,
            EffectClass::ProcessIo,
            EffectClass::Unknown,
        ]
        .into_iter()
        .filter(|effect| requested.contains(effect))
        .collect();
        Self {
            allowed_effect_classes: allowed,
            builtin_limits: BuiltinAdmissionLimits::default(),
        }
    }

    /// Admit only referentially transparent in-process work.
    pub fn pure_only() -> Self {
        Self::new([EffectClass::Pure])
    }

    /// Compatibility policy for trusted registries. Callers crossing a trust
    /// boundary should generally enumerate their grants with [`Self::new`].
    pub fn allow_all() -> Self {
        Self::new([
            EffectClass::Pure,
            EffectClass::ProcessIo,
            EffectClass::Unknown,
        ])
    }

    pub fn with_builtin_limits(mut self, limits: BuiltinAdmissionLimits) -> Self {
        self.builtin_limits = limits;
        self
    }

    pub fn allowed_effect_classes(&self) -> &[EffectClass] {
        &self.allowed_effect_classes
    }

    pub const fn builtin_limits(&self) -> BuiltinAdmissionLimits {
        self.builtin_limits
    }

    pub fn allows_effect(&self, effect: EffectClass) -> bool {
        self.allowed_effect_classes.contains(&effect)
    }
}

impl Default for AdmissionPolicy {
    fn default() -> Self {
        Self::allow_all()
    }
}

impl Registry {
    /// Admit duplicate-safe raw JSON against this registry and policy.
    ///
    /// Grammar issues are returned before registry projection because an
    /// invalid raw document has no trustworthy typed tree to walk.
    pub fn admit_source(
        &self,
        source: &[u8],
        policy: &AdmissionPolicy,
    ) -> Result<AdmittedNode, Vec<MetaIssue>> {
        let admitted = AdmittedNode::admit_source(source)?;
        let issues = validate_node_with_policy(self, admitted.as_node(), policy);
        if issues.is_empty() {
            Ok(admitted)
        } else {
            Err(issues)
        }
    }
}

pub(crate) fn validate_node_with_policy(
    registry: &Registry,
    node: &Node,
    policy: &AdmissionPolicy,
) -> Vec<MetaIssue> {
    let mut issues = Vec::new();
    walk_node(node, "", registry, policy, &mut issues);
    let mut seen = HashSet::with_capacity(issues.len());
    issues.retain(|issue| seen.insert(issue.clone()));
    issues
}

fn walk_node(
    node: &Node,
    pointer: &str,
    registry: &Registry,
    policy: &AdmissionPolicy,
    issues: &mut Vec<MetaIssue>,
) {
    let verify_pointer = child_pointer(pointer, "verify");
    for (index, spec) in node.verify.iter().enumerate() {
        walk_spec(
            spec,
            &node.ty,
            &child_pointer(&verify_pointer, &index.to_string()),
            registry,
            policy,
            issues,
        );
    }

    match &node.ty {
        Type::Object { fields, .. } => {
            let fields_pointer = child_pointer(pointer, "fields");
            for (name, field) in fields {
                walk_node(
                    &field.node,
                    &child_pointer(&fields_pointer, name),
                    registry,
                    policy,
                    issues,
                );
            }
        }
        Type::Array { item, .. } => walk_node(
            item,
            &child_pointer(pointer, "item"),
            registry,
            policy,
            issues,
        ),
        Type::Union { variants } => {
            let variants_pointer = child_pointer(pointer, "variants");
            for (index, variant) in variants.iter().enumerate() {
                walk_node(
                    variant,
                    &child_pointer(&variants_pointer, &index.to_string()),
                    registry,
                    policy,
                    issues,
                );
            }
        }
        _ => {}
    }
}

fn walk_spec(
    spec: &VerifierSpec,
    input_type: &Type,
    pointer: &str,
    registry: &Registry,
    policy: &AdmissionPolicy,
    issues: &mut Vec<MetaIssue>,
) {
    match spec {
        VerifierSpec::AllOf { all_of } => {
            let children_pointer = child_pointer(pointer, "all_of");
            for (index, child) in all_of.iter().enumerate() {
                walk_spec(
                    child,
                    input_type,
                    &child_pointer(&children_pointer, &index.to_string()),
                    registry,
                    policy,
                    issues,
                );
            }
        }
        VerifierSpec::AnyOf { any_of } => {
            let children_pointer = child_pointer(pointer, "any_of");
            for (index, child) in any_of.iter().enumerate() {
                walk_spec(
                    child,
                    input_type,
                    &child_pointer(&children_pointer, &index.to_string()),
                    registry,
                    policy,
                    issues,
                );
            }
        }
        VerifierSpec::Not { not, .. } => walk_spec(
            not,
            input_type,
            &child_pointer(pointer, "not"),
            registry,
            policy,
            issues,
        ),
        VerifierSpec::Leaf(leaf) => {
            let ext_pointer = child_pointer(pointer, "ext");
            let Some(declaration) = registry.decl(&leaf.ext) else {
                issues.push(MetaIssue::new(
                    MetaIssueCode::UnknownExtension,
                    ext_pointer,
                    format!("extension {:?} is not registered", leaf.ext),
                ));
                return;
            };

            if !policy.allows_effect(declaration.effect_class) {
                issues.push(MetaIssue::new(
                    MetaIssueCode::EffectMismatch,
                    &ext_pointer,
                    format!(
                        "extension {:?} has effect {:?}, which this boundary does not grant",
                        leaf.ext, declaration.effect_class
                    ),
                ));
            }
            if !declaration.accepted_input.accepts_type(input_type) {
                issues.push(MetaIssue::new(
                    MetaIssueCode::InputDomainMismatch,
                    &ext_pointer,
                    format!(
                        "extension {:?} does not accept node type {:?}",
                        leaf.ext,
                        input_type.name()
                    ),
                ));
            }
            if leaf.sampling.is_some() && declaration.determinism == Determinism::Deterministic {
                issues.push(MetaIssue::new(
                    MetaIssueCode::InvalidSampling,
                    child_pointer(pointer, "sampling"),
                    format!(
                        "deterministic extension {:?} must not declare sampling",
                        leaf.ext
                    ),
                ));
            }

            let config_pointer = child_pointer(pointer, "config");
            let mut config_issues =
                declaration
                    .config_schema
                    .as_ref()
                    .map_or_else(Vec::new, |schema| {
                        validate_config_structure_with_env_holes(
                            schema,
                            &leaf.config,
                            &config_pointer,
                        )
                    });
            let config_structure_ok = config_issues.is_empty();
            issues.append(&mut config_issues);

            apply_builtin_limits(
                &leaf.ext,
                &declaration.semantic_revision,
                &leaf.config,
                &config_pointer,
                policy.builtin_limits,
                issues,
            );

            if !contains_env_ref(&leaf.config) && config_structure_ok {
                if let Err(detail) = declaration.preflight_config(&leaf.config) {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::ConfigSemantics,
                        config_pointer,
                        detail,
                    ));
                }
            }
        }
    }
}

fn apply_builtin_limits(
    extension: &str,
    semantic_revision: &str,
    config: &Value,
    config_pointer: &str,
    limits: BuiltinAdmissionLimits,
    issues: &mut Vec<MetaIssue>,
) {
    if extension == "one_of" && semantic_revision == "selta.builtin.one_of.v1" {
        if let (Some(max), Some(values)) = (
            limits.max_one_of_values,
            config.get("values").and_then(Value::as_array),
        ) {
            if values.len() > max {
                issues.push(MetaIssue::new(
                    MetaIssueCode::ResourceLimitExceeded,
                    child_pointer(config_pointer, "values"),
                    format!(
                        "one_of has {} values; policy maximum is {max}",
                        values.len()
                    ),
                ));
            }
        }
    }
    if extension == "regex" && semantic_revision == "selta.builtin.regex.v1" {
        if let (Some(max), Some(pattern)) = (
            limits.max_regex_pattern_bytes,
            config.get("pattern").and_then(Value::as_str),
        ) {
            if pattern.len() > max {
                issues.push(MetaIssue::new(
                    MetaIssueCode::ResourceLimitExceeded,
                    child_pointer(config_pointer, "pattern"),
                    format!(
                        "regex pattern has {} UTF-8 bytes; policy maximum is {max}",
                        pattern.len()
                    ),
                ));
            }
        }
    }
}

pub(crate) fn first_verifier_pointer(node: &Node, pointer: &str) -> Option<String> {
    if !node.verify.is_empty() {
        return Some(child_pointer(pointer, "verify"));
    }
    match &node.ty {
        Type::Object { fields, .. } => {
            let fields_pointer = child_pointer(pointer, "fields");
            fields.iter().find_map(|(name, field)| {
                first_verifier_pointer(&field.node, &child_pointer(&fields_pointer, name))
            })
        }
        Type::Array { item, .. } => first_verifier_pointer(item, &child_pointer(pointer, "item")),
        Type::Union { variants } => {
            let variants_pointer = child_pointer(pointer, "variants");
            variants.iter().enumerate().find_map(|(index, variant)| {
                first_verifier_pointer(
                    variant,
                    &child_pointer(&variants_pointer, &index.to_string()),
                )
            })
        }
        _ => None,
    }
}
