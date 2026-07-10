//! Monitoring at the contract boundary (docs/06 §Monitoring): the engine
//! reports exactly what crosses it — calls, outcomes, latency, usage, vote
//! tallies. Aggregation lives with the caller; the embedded default is none.

use crate::host::WireUsage;
use crate::verdict::VoteTally;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallOutcome {
    Ok,
    Error,
    Timeout,
}

pub trait Monitor: Send + Sync {
    /// One verifier execution crossed the boundary.
    fn call(&self, ext: &str, outcome: CallOutcome, elapsed_ms: u64, usage: Option<&WireUsage>);

    /// One sampling round settled; agreement is derived from the tally.
    fn votes(&self, ext: &str, tally: &VoteTally);
}
