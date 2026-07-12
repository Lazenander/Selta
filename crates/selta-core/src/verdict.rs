//! The K3 verdict algebra and the evidence types: deltas, notices, check errors.
//!
//! Order: `Fail < Inconclusive < Pass`. `and` is min, `or` is max, `not` swaps
//! the extremes and fixes `Inconclusive` (docs/03, docs/08).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

/// Presence of support and refutation for one semantic proposition. This is
/// FOUR, not a confidence scale: `Both` and `Neither` remain distinct even
/// though the cautious legacy projection maps both to `Inconclusive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Neither,
    SupportOnly,
    RefuteOnly,
    Both,
}

impl EvidenceState {
    pub const fn from_presence(support: bool, refute: bool) -> Self {
        match (support, refute) {
            (false, false) => Self::Neither,
            (true, false) => Self::SupportOnly,
            (false, true) => Self::RefuteOnly,
            (true, true) => Self::Both,
        }
    }

    pub const fn supports(self) -> bool {
        matches!(self, Self::SupportOnly | Self::Both)
    }

    pub const fn refutes(self) -> bool {
        matches!(self, Self::RefuteOnly | Self::Both)
    }

    /// Knowledge join for repeated assessments of the same proposition.
    pub const fn join(self, other: Self) -> Self {
        Self::from_presence(
            self.supports() || other.supports(),
            self.refutes() || other.refutes(),
        )
    }

    /// Four-valued conjunction: support requires both operands; either
    /// refutation refutes the conjunction.
    pub const fn conjoin(self, other: Self) -> Self {
        Self::from_presence(
            self.supports() && other.supports(),
            self.refutes() || other.refutes(),
        )
    }

    /// Four-valued disjunction: either support supports the disjunction;
    /// refutation requires both operands.
    pub const fn disjoin(self, other: Self) -> Self {
        Self::from_presence(
            self.supports() || other.supports(),
            self.refutes() && other.refutes(),
        )
    }

    pub const fn negate(self) -> Self {
        Self::from_presence(self.refutes(), self.supports())
    }

    pub const fn project(self) -> Verdict {
        match self {
            Self::SupportOnly => Verdict::Pass,
            Self::RefuteOnly => Verdict::Fail,
            Self::Neither | Self::Both => Verdict::Inconclusive,
        }
    }
}

impl Verdict {
    fn rank(self) -> u8 {
        match self {
            Verdict::Fail => 0,
            Verdict::Inconclusive => 1,
            Verdict::Pass => 2,
        }
    }

    pub fn and(self, other: Verdict) -> Verdict {
        if self.rank() <= other.rank() {
            self
        } else {
            other
        }
    }

    pub fn or(self, other: Verdict) -> Verdict {
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }

    pub fn negate(self) -> Verdict {
        match self {
            Verdict::Pass => Verdict::Fail,
            Verdict::Fail => Verdict::Pass,
            Verdict::Inconclusive => Verdict::Inconclusive,
        }
    }
}

/// Sorted `Structure < Constraint < Semantic`; the derive order is the report order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaKind {
    Structure,
    Constraint,
    Semantic,
}

/// The atom of the system: the difference between the given value and an
/// acceptable one (docs/04).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub path: String,
    pub kind: DeltaKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub votes: Option<VoteTally>,
}

impl Delta {
    pub fn structure(path: String, message: String) -> Self {
        Delta {
            path,
            kind: DeltaKind::Structure,
            message,
            expected: None,
            actual: None,
            source: "structure".to_string(),
            data: None,
            votes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notice {
    pub path: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckError {
    pub path: String,
    pub source: String,
    pub error: String,
    #[serde(default)]
    pub samples_lost: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteTally {
    pub pass: u32,
    pub fail: u32,
    pub errors: u32,
    pub samples: u32,
}
