//! Proof-oriented, additive admission for untrusted Selta schema sources.
//!
//! `Node::from_value` remains the v0.1 compatibility parser. It intentionally
//! carries no admission claim. This module owns the closed raw grammar and is
//! the only constructor for [`AdmittedNode`].

use std::collections::HashSet;
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::schema::{Node, Type};

const DUPLICATE_MARKER: &str = "__SELTA_DUPLICATE_HEX__";

pub const SELTA_SCHEMA_LANGUAGE_REVISION_V1: &str = "selta.schema-language/1";
pub const SELTA_SCHEMA_LANGUAGE_REVISION_V2: &str = "selta.schema-language/2";
pub const SELTA_META_VALIDATOR_REVISION_V1: &str = "selta.meta-validator/1";
pub const SELTA_META_VALIDATOR_REVISION_V2: &str = "selta.meta-validator/2";

/// Compatibility aliases remain permanently bound to Selta 0.1. Callers must
/// select revision 2 explicitly with [`AdmissionProfile::Selta2`].
pub const SELTA_SCHEMA_LANGUAGE_REVISION: &str = SELTA_SCHEMA_LANGUAGE_REVISION_V1;
pub const SELTA_META_VALIDATOR_REVISION: &str = SELTA_META_VALIDATOR_REVISION_V1;

/// A closed schema-language and meta-validator pair. Revisions cannot be mixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdmissionProfile {
    Selta1,
    Selta2,
}

impl AdmissionProfile {
    pub const fn schema_language_revision(self) -> &'static str {
        match self {
            Self::Selta1 => SELTA_SCHEMA_LANGUAGE_REVISION_V1,
            Self::Selta2 => SELTA_SCHEMA_LANGUAGE_REVISION_V2,
        }
    }

    pub const fn meta_validator_revision(self) -> &'static str {
        match self {
            Self::Selta1 => SELTA_META_VALIDATOR_REVISION_V1,
            Self::Selta2 => SELTA_META_VALIDATOR_REVISION_V2,
        }
    }
}

/// Stable machine-readable classes emitted by strict schema admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MetaIssueCode {
    InvalidJson,
    DuplicateObjectKey,
    ExpectedObject,
    MissingProperty,
    UnexpectedProperty,
    TypeMismatch,
    UnknownNodeType,
    RequiredOutsideField,
    VerifierDiscriminant,
    VoteDiscriminant,
    EmptyCollection,
    InvalidBounds,
    InvalidSampling,
    InvalidEvidence,
    InvalidEnvRef,
    ConfigStructure,
    UnknownExtension,
    EffectMismatch,
    InputDomainMismatch,
    ConfigSemantics,
    ResourceLimitExceeded,
    InvalidDeclarationSchema,
    ProjectionMismatch,
}

impl MetaIssueCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "INVALID_JSON",
            Self::DuplicateObjectKey => "DUPLICATE_OBJECT_KEY",
            Self::ExpectedObject => "EXPECTED_OBJECT",
            Self::MissingProperty => "MISSING_PROPERTY",
            Self::UnexpectedProperty => "UNEXPECTED_PROPERTY",
            Self::TypeMismatch => "TYPE_MISMATCH",
            Self::UnknownNodeType => "UNKNOWN_NODE_TYPE",
            Self::RequiredOutsideField => "REQUIRED_OUTSIDE_FIELD",
            Self::VerifierDiscriminant => "VERIFIER_DISCRIMINANT",
            Self::VoteDiscriminant => "VOTE_DISCRIMINANT",
            Self::EmptyCollection => "EMPTY_COLLECTION",
            Self::InvalidBounds => "INVALID_BOUNDS",
            Self::InvalidSampling => "INVALID_SAMPLING",
            Self::InvalidEvidence => "INVALID_EVIDENCE",
            Self::InvalidEnvRef => "INVALID_ENV_REF",
            Self::ConfigStructure => "CONFIG_STRUCTURE",
            Self::UnknownExtension => "UNKNOWN_EXTENSION",
            Self::EffectMismatch => "EFFECT_MISMATCH",
            Self::InputDomainMismatch => "INPUT_DOMAIN_MISMATCH",
            Self::ConfigSemantics => "CONFIG_SEMANTICS",
            Self::ResourceLimitExceeded => "RESOURCE_LIMIT_EXCEEDED",
            Self::InvalidDeclarationSchema => "INVALID_DECLARATION_SCHEMA",
            Self::ProjectionMismatch => "PROJECTION_MISMATCH",
        }
    }
}

impl fmt::Display for MetaIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One stable issue code at one RFC 6901 JSON pointer. `detail` is diagnostic
/// prose; consumers branch on `code`, never by parsing it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaIssue {
    pub code: MetaIssueCode,
    pub pointer: String,
    pub detail: String,
}

impl MetaIssue {
    pub fn new(code: MetaIssueCode, pointer: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code,
            pointer: pointer.into(),
            detail: detail.into(),
        }
    }
}

/// A schema that passed duplicate-safe raw parsing and the complete closed H1
/// grammar. Its fields are deliberately private: a compatibility `Node` is not
/// an admission proof merely because it has the same Rust shape.
#[derive(Debug, Clone)]
pub struct AdmittedNode {
    node: Node,
    profile: AdmissionProfile,
}

impl AdmittedNode {
    /// Admit an untrusted raw JSON schema without extension-specific config
    /// policy. Exact `$env` syntax is still checked recursively.
    pub fn admit_source(source: &[u8]) -> Result<Self, Vec<MetaIssue>> {
        Self::admit_source_at(source, AdmissionProfile::Selta1)
    }

    pub fn admit_source_at(
        source: &[u8],
        profile: AdmissionProfile,
    ) -> Result<Self, Vec<MetaIssue>> {
        admit_source(source, None, profile)
    }

    /// Admit an untrusted schema and let an owning registry/domain add pure
    /// config-structure issues without weakening the closed core grammar.
    pub fn admit_source_with_config_validator(
        source: &[u8],
        validator: &dyn ConfigStructureValidator,
    ) -> Result<Self, Vec<MetaIssue>> {
        Self::admit_source_with_config_validator_at(source, validator, AdmissionProfile::Selta1)
    }

    pub fn admit_source_with_config_validator_at(
        source: &[u8],
        validator: &dyn ConfigStructureValidator,
        profile: AdmissionProfile,
    ) -> Result<Self, Vec<MetaIssue>> {
        admit_source(source, Some(validator), profile)
    }

    pub fn as_node(&self) -> &Node {
        &self.node
    }

    pub fn into_node(self) -> Node {
        self.node
    }

    pub const fn profile(&self) -> AdmissionProfile {
        self.profile
    }
}

/// Optional seam for extension-owned structural config admission. Builtin
/// semantic policy, extension existence, and effect/input compatibility are
/// intentionally outside this slice.
pub trait ConfigStructureValidator {
    fn validate_config(&self, extension: &str, config: &Value, pointer: &str) -> Vec<MetaIssue>;
}

impl<F> ConfigStructureValidator for F
where
    F: Fn(&str, &Value, &str) -> Vec<MetaIssue>,
{
    fn validate_config(&self, extension: &str, config: &Value, pointer: &str) -> Vec<MetaIssue> {
        self(extension, config, pointer)
    }
}

/// Structure-check a config against a declared node while treating a valid
/// exact `$env` hole as a value to be checked after resolution. Unlike the
/// legacy all-or-nothing `contains_env_ref` shortcut, every literal sibling is
/// still checked.
pub fn validate_config_structure_with_env_holes(
    schema: &Node,
    config: &Value,
    pointer: &str,
) -> Vec<MetaIssue> {
    let mut issues = Vec::new();
    validate_env_refs(config, pointer, &mut issues);
    validate_config_value(schema, config, pointer, &mut issues);
    issues
}

fn admit_source(
    source: &[u8],
    validator: Option<&dyn ConfigStructureValidator>,
    profile: AdmissionProfile,
) -> Result<AdmittedNode, Vec<MetaIssue>> {
    let value = parse_unique_json(source)?;
    let mut issues = Vec::new();
    validate_node(&value, "", false, validator, profile, &mut issues);
    let mut seen = HashSet::with_capacity(issues.len());
    issues.retain(|issue| seen.insert(issue.clone()));
    if !issues.is_empty() {
        return Err(issues);
    }
    match Node::from_value(value) {
        Ok(node) => Ok(AdmittedNode { node, profile }),
        Err(error) => Err(vec![MetaIssue::new(
            MetaIssueCode::ProjectionMismatch,
            "",
            format!("closed grammar did not project to Node: {error}"),
        )]),
    }
}

fn validate_node(
    value: &Value,
    pointer: &str,
    field_position: bool,
    validator: Option<&dyn ConfigStructureValidator>,
    profile: AdmissionProfile,
    issues: &mut Vec<MetaIssue>,
) {
    let Some(map) = value.as_object() else {
        issues.push(MetaIssue::new(
            MetaIssueCode::ExpectedObject,
            pointer,
            "a Selta node must be an object",
        ));
        return;
    };

    if map.contains_key("required") && !field_position {
        issues.push(MetaIssue::new(
            MetaIssueCode::RequiredOutsideField,
            child_pointer(pointer, "required"),
            "required is legal only on an object field node",
        ));
    }
    if field_position {
        validate_optional_bool(map, "required", pointer, issues);
    }
    validate_optional_string(map, "description", pointer, false, issues);

    if let Some(verify) = map.get("verify") {
        let verify_pointer = child_pointer(pointer, "verify");
        match verify.as_array() {
            Some(specs) => {
                for (index, spec) in specs.iter().enumerate() {
                    let _ = validate_verifier(
                        spec,
                        &child_pointer(&verify_pointer, &index.to_string()),
                        validator,
                        profile,
                        issues,
                    );
                }
            }
            None => issues.push(type_issue(&verify_pointer, "array")),
        }
    }

    let type_pointer = child_pointer(pointer, "type");
    let Some(type_value) = map.get("type") else {
        issues.push(MetaIssue::new(
            MetaIssueCode::MissingProperty,
            &type_pointer,
            "node.type is required",
        ));
        reject_unknown_keys(map, pointer, &common_node_keys(field_position), issues);
        return;
    };
    let Some(type_name) = type_value.as_str() else {
        issues.push(type_issue(&type_pointer, "string"));
        reject_unknown_keys(map, pointer, &common_node_keys(field_position), issues);
        return;
    };

    let mut allowed = common_node_keys(field_position);
    match type_name {
        "null" | "bool" | "int" | "float" | "str" | "any" => {}
        "object" => {
            allowed.extend(["fields", "open"]);
            validate_optional_bool(map, "open", pointer, issues);
            if let Some(fields) = map.get("fields") {
                let fields_pointer = child_pointer(pointer, "fields");
                match fields.as_object() {
                    Some(fields) => {
                        for (name, field) in fields {
                            validate_node(
                                field,
                                &child_pointer(&fields_pointer, name),
                                true,
                                validator,
                                profile,
                                issues,
                            );
                        }
                    }
                    None => issues.push(type_issue(&fields_pointer, "object")),
                }
            }
        }
        "array" => {
            allowed.extend(["item", "len"]);
            let item_pointer = child_pointer(pointer, "item");
            match map.get("item") {
                Some(item) => validate_node(item, &item_pointer, false, validator, profile, issues),
                None => issues.push(MetaIssue::new(
                    MetaIssueCode::MissingProperty,
                    item_pointer,
                    "array.item is required",
                )),
            }
            if let Some(bounds) = map.get("len") {
                validate_len_bounds(bounds, &child_pointer(pointer, "len"), issues);
            }
        }
        "union" => {
            allowed.push("variants");
            let variants_pointer = child_pointer(pointer, "variants");
            match map.get("variants") {
                Some(Value::Array(variants)) => {
                    if variants.is_empty() {
                        issues.push(MetaIssue::new(
                            MetaIssueCode::EmptyCollection,
                            &variants_pointer,
                            "union.variants must not be empty",
                        ));
                    }
                    for (index, variant) in variants.iter().enumerate() {
                        validate_node(
                            variant,
                            &child_pointer(&variants_pointer, &index.to_string()),
                            false,
                            validator,
                            profile,
                            issues,
                        );
                    }
                }
                Some(_) => issues.push(type_issue(&variants_pointer, "array")),
                None => issues.push(MetaIssue::new(
                    MetaIssueCode::MissingProperty,
                    variants_pointer,
                    "union.variants is required",
                )),
            }
        }
        other => issues.push(MetaIssue::new(
            MetaIssueCode::UnknownNodeType,
            type_pointer,
            format!("unknown node type {other:?}"),
        )),
    }
    reject_unknown_keys(map, pointer, &allowed, issues);
}

fn common_node_keys(_field_position: bool) -> Vec<&'static str> {
    // Recognize `required` everywhere so an illegal position produces only
    // the dedicated REQUIRED_OUTSIDE_FIELD issue. `validate_node` still
    // admits it exclusively at object-field positions.
    vec!["type", "verify", "description", "required"]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceUse {
    Legacy,
    Cautious,
    Invalid,
}

fn validate_verifier(
    value: &Value,
    pointer: &str,
    validator: Option<&dyn ConfigStructureValidator>,
    profile: AdmissionProfile,
    issues: &mut Vec<MetaIssue>,
) -> EvidenceUse {
    let Some(map) = value.as_object() else {
        issues.push(MetaIssue::new(
            MetaIssueCode::ExpectedObject,
            pointer,
            "a verifier spec must be an object",
        ));
        return EvidenceUse::Invalid;
    };
    let discriminants = ["ext", "all_of", "any_of", "not"]
        .into_iter()
        .filter(|key| map.contains_key(*key))
        .collect::<Vec<_>>();
    if discriminants.len() != 1 {
        issues.push(MetaIssue::new(
            MetaIssueCode::VerifierDiscriminant,
            pointer,
            "a verifier spec must contain exactly one of ext/all_of/any_of/not",
        ));
        let allowed = match profile {
            AdmissionProfile::Selta1 => &[
                "ext", "config", "sampling", "all_of", "any_of", "not", "message",
            ][..],
            AdmissionProfile::Selta2 => &[
                "ext", "config", "evidence", "sampling", "all_of", "any_of", "not", "message",
            ][..],
        };
        reject_unknown_keys(map, pointer, allowed, issues);
        return EvidenceUse::Invalid;
    }

    match discriminants[0] {
        "ext" => {
            let allowed = match profile {
                AdmissionProfile::Selta1 => &["ext", "config", "sampling"][..],
                AdmissionProfile::Selta2 => &["ext", "config", "evidence", "sampling"][..],
            };
            reject_unknown_keys(map, pointer, allowed, issues);
            let ext_pointer = child_pointer(pointer, "ext");
            let extension = match map.get("ext").and_then(Value::as_str) {
                Some(extension) if !extension.trim().is_empty() => Some(extension),
                Some(_) => {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::TypeMismatch,
                        ext_pointer,
                        "extension name must not be empty",
                    ));
                    None
                }
                None => {
                    issues.push(type_issue(&ext_pointer, "non-empty string"));
                    None
                }
            };
            let default_config = Value::Object(Map::new());
            let config = map.get("config").unwrap_or(&default_config);
            let config_pointer = child_pointer(pointer, "config");
            validate_env_refs(config, &config_pointer, issues);
            if let (Some(extension), Some(validator)) = (extension, validator) {
                issues.extend(validator.validate_config(extension, config, &config_pointer));
            }
            let evidence = match profile {
                AdmissionProfile::Selta1 => EvidenceUse::Legacy,
                AdmissionProfile::Selta2 => validate_evidence(map.get("evidence"), pointer, issues),
            };
            if let Some(sampling) = map.get("sampling") {
                validate_sampling(sampling, &child_pointer(pointer, "sampling"), issues);
                if evidence == EvidenceUse::Cautious
                    && sampling
                        .as_object()
                        .is_some_and(|sampling| sampling.contains_key("vote"))
                {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::InvalidEvidence,
                        child_pointer(&child_pointer(pointer, "sampling"), "vote"),
                        "sampling.vote must be omitted when evidence is cautious",
                    ));
                }
            }
            evidence
        }
        "all_of" | "any_of" => {
            let key = discriminants[0];
            reject_unknown_keys(map, pointer, &[key], issues);
            let children_pointer = child_pointer(pointer, key);
            match map.get(key) {
                Some(Value::Array(children)) => {
                    if children.is_empty() {
                        issues.push(MetaIssue::new(
                            MetaIssueCode::EmptyCollection,
                            &children_pointer,
                            format!("{key} must not be empty"),
                        ));
                    }
                    let mut modes = Vec::with_capacity(children.len());
                    for (index, child) in children.iter().enumerate() {
                        modes.push(validate_verifier(
                            child,
                            &child_pointer(&children_pointer, &index.to_string()),
                            validator,
                            profile,
                            issues,
                        ));
                    }
                    combine_evidence_modes(&modes, pointer, issues)
                }
                Some(_) => {
                    issues.push(type_issue(&children_pointer, "array"));
                    EvidenceUse::Invalid
                }
                None => unreachable!("discriminant was selected from an existing key"),
            }
        }
        "not" => {
            reject_unknown_keys(map, pointer, &["not", "message"], issues);
            let evidence = validate_verifier(
                &map["not"],
                &child_pointer(pointer, "not"),
                validator,
                profile,
                issues,
            );
            let message_pointer = child_pointer(pointer, "message");
            match map.get("message").and_then(Value::as_str) {
                Some(message) if !message.trim().is_empty() => {}
                Some(_) => issues.push(MetaIssue::new(
                    MetaIssueCode::EmptyCollection,
                    message_pointer,
                    "not.message must not be empty",
                )),
                None if map.contains_key("message") => {
                    issues.push(type_issue(&message_pointer, "non-empty string"))
                }
                None => issues.push(MetaIssue::new(
                    MetaIssueCode::MissingProperty,
                    message_pointer,
                    "not.message is required",
                )),
            }
            evidence
        }
        _ => unreachable!("discriminant is from the closed list"),
    }
}

fn validate_evidence(
    value: Option<&Value>,
    pointer: &str,
    issues: &mut Vec<MetaIssue>,
) -> EvidenceUse {
    let Some(value) = value else {
        return EvidenceUse::Legacy;
    };
    if value.as_str() == Some("cautious") {
        return EvidenceUse::Cautious;
    }
    issues.push(MetaIssue::new(
        MetaIssueCode::InvalidEvidence,
        child_pointer(pointer, "evidence"),
        "evidence must be the string \"cautious\"",
    ));
    EvidenceUse::Invalid
}

fn combine_evidence_modes(
    modes: &[EvidenceUse],
    pointer: &str,
    issues: &mut Vec<MetaIssue>,
) -> EvidenceUse {
    let legacy = modes.contains(&EvidenceUse::Legacy);
    let cautious = modes.contains(&EvidenceUse::Cautious);
    if legacy && cautious {
        issues.push(MetaIssue::new(
            MetaIssueCode::InvalidEvidence,
            pointer,
            "a verifier combinator subtree must be wholly legacy or wholly cautious",
        ));
        return EvidenceUse::Invalid;
    }
    if modes.contains(&EvidenceUse::Invalid) {
        EvidenceUse::Invalid
    } else if cautious {
        EvidenceUse::Cautious
    } else {
        EvidenceUse::Legacy
    }
}

fn validate_sampling(value: &Value, pointer: &str, issues: &mut Vec<MetaIssue>) {
    let Some(map) = value.as_object() else {
        issues.push(MetaIssue::new(
            MetaIssueCode::ExpectedObject,
            pointer,
            "sampling must be an object",
        ));
        return;
    };
    reject_unknown_keys(
        map,
        pointer,
        &["samples", "vote", "depth", "min_valid"],
        issues,
    );
    let samples = validate_optional_u32(map, "samples", pointer, issues).unwrap_or(3);
    if samples == 0 {
        issues.push(MetaIssue::new(
            MetaIssueCode::InvalidSampling,
            child_pointer(pointer, "samples"),
            "sampling.samples must be at least 1",
        ));
    }
    validate_optional_u32(map, "depth", pointer, issues);
    if let Some(min_valid) = validate_optional_u32(map, "min_valid", pointer, issues) {
        if min_valid == 0 || min_valid > samples {
            issues.push(MetaIssue::new(
                MetaIssueCode::InvalidSampling,
                child_pointer(pointer, "min_valid"),
                "sampling.min_valid must be in 1..=samples",
            ));
        }
    }
    if let Some(vote) = map.get("vote") {
        validate_vote(vote, &child_pointer(pointer, "vote"), samples, issues);
    }
}

fn validate_vote(value: &Value, pointer: &str, samples: u32, issues: &mut Vec<MetaIssue>) {
    if let Some(name) = value.as_str() {
        if !matches!(name, "majority" | "unanimous") {
            issues.push(MetaIssue::new(
                MetaIssueCode::VoteDiscriminant,
                pointer,
                "vote string must be majority or unanimous",
            ));
        }
        return;
    }
    let Some(map) = value.as_object() else {
        issues.push(MetaIssue::new(
            MetaIssueCode::TypeMismatch,
            pointer,
            "vote must be a named string or a policy object",
        ));
        return;
    };
    let discriminants = ["at_least", "ratio"]
        .into_iter()
        .filter(|key| map.contains_key(*key))
        .collect::<Vec<_>>();
    if discriminants.len() != 1 {
        issues.push(MetaIssue::new(
            MetaIssueCode::VoteDiscriminant,
            pointer,
            "vote object must contain exactly one of at_least/ratio",
        ));
        reject_unknown_keys(map, pointer, &["at_least", "ratio"], issues);
        return;
    }
    reject_unknown_keys(map, pointer, &[discriminants[0]], issues);
    match discriminants[0] {
        "at_least" => {
            let at_pointer = child_pointer(pointer, "at_least");
            match value_as_u32(&map["at_least"]) {
                Some(at_least) if at_least > 0 && at_least <= samples => {}
                Some(_) => issues.push(MetaIssue::new(
                    MetaIssueCode::InvalidSampling,
                    at_pointer,
                    "vote.at_least must be in 1..=samples",
                )),
                None => issues.push(type_issue(&at_pointer, "non-negative u32 integer")),
            }
        }
        "ratio" => {
            let ratio_pointer = child_pointer(pointer, "ratio");
            match map["ratio"].as_f64() {
                Some(ratio) if ratio > 0.0 && ratio <= 1.0 => {}
                Some(_) => issues.push(MetaIssue::new(
                    MetaIssueCode::InvalidSampling,
                    ratio_pointer,
                    "vote.ratio must be in (0, 1]",
                )),
                None => issues.push(type_issue(&ratio_pointer, "number")),
            }
        }
        _ => unreachable!("vote discriminant is from the closed list"),
    }
}

fn validate_len_bounds(value: &Value, pointer: &str, issues: &mut Vec<MetaIssue>) {
    let Some(map) = value.as_object() else {
        issues.push(MetaIssue::new(
            MetaIssueCode::ExpectedObject,
            pointer,
            "array.len must be an object",
        ));
        return;
    };
    reject_unknown_keys(map, pointer, &["min", "max"], issues);
    if !map.contains_key("min") && !map.contains_key("max") {
        issues.push(MetaIssue::new(
            MetaIssueCode::EmptyCollection,
            pointer,
            "array.len must contain min, max, or both",
        ));
    }
    let min = validate_optional_usize(map, "min", pointer, issues);
    let max = validate_optional_usize(map, "max", pointer, issues);
    if matches!((min, max), (Some(min), Some(max)) if min > max) {
        issues.push(MetaIssue::new(
            MetaIssueCode::InvalidBounds,
            pointer,
            "array.len.min must not exceed array.len.max",
        ));
    }
}

fn validate_env_refs(value: &Value, pointer: &str, issues: &mut Vec<MetaIssue>) {
    match value {
        Value::Object(map) => {
            if let Some(path) = map.get("$env") {
                if map.len() != 1 {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::InvalidEnvRef,
                        pointer,
                        "an object containing $env must contain no literal siblings",
                    ));
                }
                if !path.as_str().is_some_and(valid_env_path) {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::InvalidEnvRef,
                        child_pointer(pointer, "$env"),
                        "$env must be a non-empty dot path with no empty segment",
                    ));
                }
            }
            for (key, child) in map {
                if key != "$env" {
                    validate_env_refs(child, &child_pointer(pointer, key), issues);
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_env_refs(item, &child_pointer(pointer, &index.to_string()), issues);
            }
        }
        _ => {}
    }
}

fn valid_env_path(path: &str) -> bool {
    !path.is_empty()
        && path.trim() == path
        && path
            .split('.')
            .all(|segment| !segment.is_empty() && segment.trim() == segment)
}

fn validate_config_value(schema: &Node, value: &Value, pointer: &str, issues: &mut Vec<MetaIssue>) {
    if is_exact_env_ref(value) {
        return;
    }
    match &schema.ty {
        Type::Null if !value.is_null() => issues.push(config_type_issue(pointer, "null")),
        Type::Bool if !value.is_boolean() => issues.push(config_type_issue(pointer, "boolean")),
        Type::Int if value.as_i64().is_none() && value.as_u64().is_none() => {
            issues.push(config_type_issue(pointer, "integer"));
        }
        Type::Float if !value.is_number() => issues.push(config_type_issue(pointer, "number")),
        Type::Str if !value.is_string() => issues.push(config_type_issue(pointer, "string")),
        Type::Any => {}
        Type::Object { fields, open } => {
            let Some(map) = value.as_object() else {
                issues.push(config_type_issue(pointer, "object"));
                return;
            };
            for (name, field) in fields {
                let child = child_pointer(pointer, name);
                match map.get(name) {
                    Some(value) => validate_config_value(&field.node, value, &child, issues),
                    None if field.required => issues.push(MetaIssue::new(
                        MetaIssueCode::ConfigStructure,
                        child,
                        "required config field is missing",
                    )),
                    None => {}
                }
            }
            if !open {
                for key in map.keys().filter(|key| !fields.contains_key(*key)) {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::ConfigStructure,
                        child_pointer(pointer, key),
                        "unexpected config field",
                    ));
                }
            }
        }
        Type::Array { item, len } => {
            let Some(items) = value.as_array() else {
                issues.push(config_type_issue(pointer, "array"));
                return;
            };
            if let Some(bounds) = len {
                if bounds.min.is_some_and(|min| items.len() < min)
                    || bounds.max.is_some_and(|max| items.len() > max)
                {
                    issues.push(MetaIssue::new(
                        MetaIssueCode::ConfigStructure,
                        pointer,
                        "config array length is outside the declared bounds",
                    ));
                }
            }
            for (index, value) in items.iter().enumerate() {
                validate_config_value(
                    item,
                    value,
                    &child_pointer(pointer, &index.to_string()),
                    issues,
                );
            }
        }
        Type::Union { variants } => {
            let mut best: Option<Vec<MetaIssue>> = None;
            for variant in variants {
                let mut candidate = Vec::new();
                validate_config_value(variant, value, pointer, &mut candidate);
                if candidate.is_empty() {
                    return;
                }
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.len() < current.len())
                {
                    best = Some(candidate);
                }
            }
            if let Some(best) = best {
                issues.extend(best);
            } else {
                issues.push(MetaIssue::new(
                    MetaIssueCode::ConfigStructure,
                    pointer,
                    "config matches no union variant",
                ));
            }
        }
        _ => {}
    }
}

fn is_exact_env_ref(value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    map.len() == 1
        && map
            .get("$env")
            .and_then(Value::as_str)
            .is_some_and(valid_env_path)
}

fn config_type_issue(pointer: &str, expected: &str) -> MetaIssue {
    MetaIssue::new(
        MetaIssueCode::ConfigStructure,
        pointer,
        format!("config value must be {expected}"),
    )
}

fn reject_unknown_keys(
    map: &Map<String, Value>,
    pointer: &str,
    allowed: &[&str],
    issues: &mut Vec<MetaIssue>,
) {
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            issues.push(MetaIssue::new(
                MetaIssueCode::UnexpectedProperty,
                child_pointer(pointer, key),
                "property is not part of this closed Selta shape",
            ));
        }
    }
}

fn validate_optional_bool(
    map: &Map<String, Value>,
    key: &str,
    pointer: &str,
    issues: &mut Vec<MetaIssue>,
) {
    if map.get(key).is_some_and(|value| !value.is_boolean()) {
        issues.push(type_issue(&child_pointer(pointer, key), "boolean"));
    }
}

fn validate_optional_string(
    map: &Map<String, Value>,
    key: &str,
    pointer: &str,
    nonempty: bool,
    issues: &mut Vec<MetaIssue>,
) {
    if let Some(value) = map.get(key) {
        match value.as_str() {
            Some(text) if !nonempty || !text.trim().is_empty() => {}
            _ => issues.push(type_issue(
                &child_pointer(pointer, key),
                if nonempty {
                    "non-empty string"
                } else {
                    "string"
                },
            )),
        }
    }
}

fn validate_optional_u32(
    map: &Map<String, Value>,
    key: &str,
    pointer: &str,
    issues: &mut Vec<MetaIssue>,
) -> Option<u32> {
    let value = map.get(key)?;
    match value_as_u32(value) {
        Some(number) => Some(number),
        None => {
            issues.push(type_issue(
                &child_pointer(pointer, key),
                "non-negative u32 integer",
            ));
            None
        }
    }
}

fn validate_optional_usize(
    map: &Map<String, Value>,
    key: &str,
    pointer: &str,
    issues: &mut Vec<MetaIssue>,
) -> Option<usize> {
    let value = map.get(key)?;
    match value
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
    {
        Some(number) => Some(number),
        None => {
            issues.push(type_issue(
                &child_pointer(pointer, key),
                "non-negative platform-size integer",
            ));
            None
        }
    }
}

fn value_as_u32(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|number| u32::try_from(number).ok())
}

fn type_issue(pointer: &str, expected: &str) -> MetaIssue {
    MetaIssue::new(
        MetaIssueCode::TypeMismatch,
        pointer,
        format!("value must be {expected}"),
    )
}

pub(crate) fn child_pointer(parent: &str, token: &str) -> String {
    let escaped = token.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

fn parse_unique_json(source: &[u8]) -> Result<Value, Vec<MetaIssue>> {
    let mut deserializer = serde_json::Deserializer::from_slice(source);
    match (UniqueValueSeed {
        pointer: String::new(),
    })
    .deserialize(&mut deserializer)
    {
        Ok(value) => match deserializer.end() {
            Ok(()) => Ok(value),
            Err(error) => Err(vec![json_issue(error)]),
        },
        Err(error) => Err(vec![json_issue(error)]),
    }
}

fn json_issue(error: serde_json::Error) -> MetaIssue {
    let message = error.to_string();
    if let Some(marker) = message.find(DUPLICATE_MARKER) {
        let encoded = message[marker + DUPLICATE_MARKER.len()..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect::<String>();
        if let Some(pointer) = decode_hex(&encoded) {
            return MetaIssue::new(
                MetaIssueCode::DuplicateObjectKey,
                pointer,
                "duplicate object key",
            );
        }
    }
    MetaIssue::new(MetaIssueCode::InvalidJson, "", message)
}

fn encode_hex(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn decode_hex(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let text = std::str::from_utf8(pair).ok()?;
        bytes.push(u8::from_str_radix(text, 16).ok()?);
    }
    String::from_utf8(bytes).ok()
}

struct UniqueValueSeed {
    pointer: String,
}

impl<'de> DeserializeSeed<'de> for UniqueValueSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueValueVisitor {
            pointer: self.pointer,
        })
    }
}

struct UniqueValueVisitor {
    pointer: String,
}

impl<'de> Visitor<'de> for UniqueValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JSON number must be finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element_seed(UniqueValueSeed {
            pointer: child_pointer(&self.pointer, &values.len().to_string()),
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut keys = HashSet::with_capacity(object.size_hint().unwrap_or(0));
        while let Some(key) = object.next_key::<String>()? {
            let pointer = child_pointer(&self.pointer, &key);
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "{DUPLICATE_MARKER}{}",
                    encode_hex(&pointer)
                )));
            }
            let value = object.next_value_seed(UniqueValueSeed { pointer })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
