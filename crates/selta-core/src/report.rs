//! The output contract (docs/04): a verdict tree plus flattened deltas.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::verdict::{CheckError, Delta, Notice, Verdict, VoteTally};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Resolved-settings fingerprint per extension that ran (docs/04):
    /// together with the pinned schema version, a verdict is attributable
    /// to the exact configuration that produced it.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub extensions: IndexMap<String, String>,
    pub verdict: Verdict,
    pub deltas: Vec<Delta>,
    pub notices: Vec<Notice>,
    pub errors: Vec<CheckError>,
    pub root: NodeResult,
    pub usage: Usage,
    pub timing: Timing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub path: String,
    pub verdict: Verdict,
    pub checks: Vec<CheckResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Children>,
}

impl NodeResult {
    /// Document-order delta collection; the caller applies the kind ordering.
    pub fn collect_deltas(&self, out: &mut Vec<Delta>) {
        for check in &self.checks {
            out.extend(check.deltas.iter().cloned());
        }
        match &self.children {
            Some(Children::Fields(fields)) => {
                for child in fields.values() {
                    child.collect_deltas(out);
                }
            }
            Some(Children::Items(items)) => {
                for child in items {
                    child.collect_deltas(out);
                }
            }
            None => {}
        }
    }

    pub fn count_deltas(&self) -> usize {
        let mut all = Vec::new();
        self.collect_deltas(&mut all);
        all.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Children {
    Fields(IndexMap<String, NodeResult>),
    Items(Vec<NodeResult>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub source: String,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deltas: Vec<Delta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub votes: Option<VoteTally>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CheckResult {
    pub fn passing(source: &str) -> Self {
        CheckResult {
            source: source.to_string(),
            verdict: Verdict::Pass,
            deltas: Vec::new(),
            votes: None,
            skipped: false,
            variant: None,
            error: None,
        }
    }

    pub fn failing(source: &str, deltas: Vec<Delta>) -> Self {
        CheckResult {
            verdict: Verdict::Fail,
            deltas,
            ..CheckResult::passing(source)
        }
    }

    pub fn erroring(source: &str, error: String) -> Self {
        CheckResult {
            verdict: Verdict::Inconclusive,
            error: Some(error),
            ..CheckResult::passing(source)
        }
    }

    pub fn skipped(source: &str) -> Self {
        CheckResult {
            verdict: Verdict::Inconclusive,
            skipped: true,
            ..CheckResult::passing(source)
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub samples: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

impl Usage {
    pub fn try_add_wire(&mut self, wire: &crate::host::WireUsage) -> Result<(), &'static str> {
        if !wire.cost_usd.is_finite() || wire.cost_usd < 0.0 {
            return Err("host usage cost must be finite and non-negative");
        }
        let input_tokens = self
            .input_tokens
            .checked_add(wire.input_tokens)
            .ok_or("host input-token usage overflow")?;
        let output_tokens = self
            .output_tokens
            .checked_add(wire.output_tokens)
            .ok_or("host output-token usage overflow")?;
        let cost_usd = self.cost_usd + wire.cost_usd;
        if !cost_usd.is_finite() {
            return Err("host cost usage overflow");
        }
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self.cost_usd = cost_usd;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    pub started: String,
    pub elapsed_ms: u64,
}
