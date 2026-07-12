//! The output contract (docs/04): a verdict tree plus flattened deltas.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::verdict::{CheckError, Delta, EvidenceState, Notice, Verdict, VoteTally};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<EvidenceSummary>,
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
            evidence: None,
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

    /// A semantic non-conclusion, distinct from an operational error and from
    /// a check skipped by the recursion budget.
    pub fn inconclusive(source: &str) -> Self {
        CheckResult {
            verdict: Verdict::Inconclusive,
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

    pub fn with_evidence(mut self, evidence: EvidenceSummary) -> Self {
        self.evidence = Some(evidence);
        self
    }
}

/// Evidence observations retained by an evidence-mode check. `state` is absent
/// when no semantic assessment completed; operational unavailability is
/// counted separately and never manufactures semantic `Neither`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<EvidenceState>,
    pub neither: u32,
    pub support_only: u32,
    pub refute_only: u32,
    pub both: u32,
    pub unavailable: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refutations: Vec<Delta>,
}

impl EvidenceSummary {
    pub fn record(
        &mut self,
        observation: EvidenceState,
        refutation: Option<Delta>,
    ) -> Result<(), &'static str> {
        if observation.refutes() != refutation.is_some() {
            return Err(if observation.refutes() {
                "refuting evidence requires a refutation delta"
            } else {
                "non-refuting evidence must not carry a refutation delta"
            });
        }
        if let Some(delta) = &refutation {
            if delta.kind != crate::verdict::DeltaKind::Semantic {
                return Err("evidence refutations must be semantic deltas");
            }
            if delta.message.trim().is_empty() {
                return Err("evidence refutation message must not be empty");
            }
        }

        let count = match observation {
            EvidenceState::Neither => &mut self.neither,
            EvidenceState::SupportOnly => &mut self.support_only,
            EvidenceState::RefuteOnly => &mut self.refute_only,
            EvidenceState::Both => &mut self.both,
        };
        let next_count = count
            .checked_add(1)
            .ok_or("evidence observation counter overflow")?;
        let next_state = Some(
            self.state
                .map_or(observation, |current| current.join(observation)),
        );

        *count = next_count;
        self.state = next_state;
        if let Some(delta) = refutation {
            if !self.refutations.contains(&delta) {
                self.refutations.push(delta);
            }
        }
        Ok(())
    }

    pub fn record_unavailable(&mut self) -> Result<(), &'static str> {
        self.record_unavailable_n(1)
    }

    pub fn record_unavailable_n(&mut self, count: u32) -> Result<(), &'static str> {
        self.unavailable = self
            .unavailable
            .checked_add(count)
            .ok_or("evidence unavailable counter overflow")?;
        Ok(())
    }

    /// Knowledge-join another summary while retaining raw observation counts
    /// and the first stable occurrence of each normalized refutation.
    pub fn checked_join(&mut self, other: &Self) -> Result<(), &'static str> {
        let neither = self
            .neither
            .checked_add(other.neither)
            .ok_or("evidence neither counter overflow")?;
        let support_only = self
            .support_only
            .checked_add(other.support_only)
            .ok_or("evidence support-only counter overflow")?;
        let refute_only = self
            .refute_only
            .checked_add(other.refute_only)
            .ok_or("evidence refute-only counter overflow")?;
        let both = self
            .both
            .checked_add(other.both)
            .ok_or("evidence both counter overflow")?;
        let unavailable = self
            .unavailable
            .checked_add(other.unavailable)
            .ok_or("evidence unavailable counter overflow")?;
        let state = match (self.state, other.state) {
            (Some(left), Some(right)) => Some(left.join(right)),
            (Some(state), None) | (None, Some(state)) => Some(state),
            (None, None) => None,
        };
        let mut refutations = self.refutations.clone();
        for delta in &other.refutations {
            if !refutations.contains(delta) {
                refutations.push(delta.clone());
            }
        }

        self.neither = neither;
        self.support_only = support_only;
        self.refute_only = refute_only;
        self.both = both;
        self.unavailable = unavailable;
        self.state = state;
        self.refutations = refutations;
        Ok(())
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
