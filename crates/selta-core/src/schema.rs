//! The schema model (docs/02): a tree of typed nodes, each optionally carrying
//! verifier specs. Pure data — the canonical representation is JSON.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    #[serde(flatten)]
    pub ty: Type,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verify: Vec<VerifierSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Node {
    pub fn from_value(value: Value) -> Result<Node, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Type {
    Null,
    Bool,
    Int,
    Float,
    Str,
    Any,
    Object {
        #[serde(default)]
        fields: IndexMap<String, Field>,
        #[serde(default)]
        open: bool,
    },
    Array {
        item: Box<Node>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        len: Option<LenBounds>,
    },
    Union {
        variants: Vec<Node>,
    },
}

impl Type {
    pub fn name(&self) -> &'static str {
        match self {
            Type::Null => "null",
            Type::Bool => "bool",
            Type::Int => "int",
            Type::Float => "float",
            Type::Str => "str",
            Type::Any => "any",
            Type::Object { .. } => "object",
            Type::Array { .. } => "array",
            Type::Union { .. } => "union",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    #[serde(flatten)]
    pub node: Node,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LenBounds {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
}

/// A leaf names an extension; combinators compose specs (docs/02).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerifierSpec {
    AllOf {
        all_of: Vec<VerifierSpec>,
    },
    AnyOf {
        any_of: Vec<VerifierSpec>,
    },
    Not {
        not: Box<VerifierSpec>,
        message: String,
    },
    Leaf(LeafSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafSpec {
    pub ext: String,
    #[serde(default = "empty_object")]
    pub config: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Sampling>,
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sampling {
    #[serde(default = "default_samples")]
    pub samples: u32,
    #[serde(default)]
    pub vote: VotePolicy,
    #[serde(default = "default_depth")]
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_valid: Option<u32>,
}

impl Default for Sampling {
    fn default() -> Self {
        Sampling {
            samples: default_samples(),
            vote: VotePolicy::default(),
            depth: default_depth(),
            min_valid: None,
        }
    }
}

impl Sampling {
    /// Quorum: explicit `min_valid`, or `ceil(samples / 2)`.
    pub fn quorum(&self) -> u32 {
        self.min_valid.unwrap_or(self.samples.div_ceil(2))
    }
}

fn default_samples() -> u32 {
    3
}

fn default_depth() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VotePolicy {
    Named(NamedVote),
    AtLeast { at_least: u32 },
    Ratio { ratio: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedVote {
    Majority,
    Unanimous,
}

impl Default for VotePolicy {
    fn default() -> Self {
        VotePolicy::Named(NamedVote::Majority)
    }
}

impl VotePolicy {
    /// Decide over valid votes only (docs/03 §Voting).
    pub fn passes(&self, pass: u32, fail: u32) -> bool {
        let valid = pass + fail;
        match self {
            VotePolicy::Named(NamedVote::Majority) => pass > fail,
            VotePolicy::Named(NamedVote::Unanimous) => fail == 0,
            VotePolicy::AtLeast { at_least } => pass >= *at_least,
            VotePolicy::Ratio { ratio } => {
                valid > 0 && (pass as f64) / (valid as f64) >= *ratio
            }
        }
    }
}
