//! Selta core: the embeddable schema layer for regulating LLM output.
//!
//! The entire contract is one function:
//! `(schema, value, env) → Report { verdict, deltas }` — see docs/01.

pub mod cache;
mod engine;
pub mod host;
pub mod intake;
pub mod meta;
pub mod monitor;
pub mod path;
pub mod registry;
pub mod report;
pub mod schema;
pub mod settings;
pub mod verdict;

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

pub use cache::{Cache, MemoryCache, NoCache};
pub use host::builtin::{CmdInput, CmdTemplate, CmdTemplates};
pub use host::rpc::{
    decl_from_manifest, initialize_over_peer, verify_over_peer, RpcHost, RpcPeer,
};
pub use host::{
    Determinism, Envelope, ExtensionDecl, ExtensionHost, HostCall, Needs, PassFail, WireDelta,
    WireUsage,
};
pub use intake::Mode;
pub use monitor::{CallOutcome, Monitor};
pub use path::Path;
pub use registry::Registry;
pub use report::{CheckResult, Children, NodeResult, Report, Timing, Usage};
pub use schema::{Field, LeafSpec, LenBounds, Node, Sampling, Type, VerifierSpec, VotePolicy};
pub use settings::{NoSettings, ResolvedSettings, SettingsResolver};
pub use verdict::{CheckError, Delta, DeltaKind, Notice, Verdict, VoteTally};

/// Everything a verification runs against, minus the request itself. The
/// registry and cache are required; settings and monitoring default to none —
/// the embedded, SQLite-like mode (docs/06).
#[derive(Clone, Copy)]
pub struct Runtime<'a> {
    pub registry: &'a Registry,
    pub cache: &'a dyn Cache,
    pub settings: &'a dyn SettingsResolver,
    pub monitor: Option<&'a dyn Monitor>,
}

impl<'a> Runtime<'a> {
    pub fn new(registry: &'a Registry, cache: &'a dyn Cache) -> Runtime<'a> {
        Runtime {
            registry,
            cache,
            settings: &NoSettings,
            monitor: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub fail_fast: bool,
    /// Request-level cap on recursion depth; leaf `sampling.depth` is clamped to it.
    pub max_depth: u32,
    /// Request-level cap on total verifier executions.
    pub max_samples: u32,
    pub deadline_ms: Option<u64>,
    pub schema_name: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Lenient,
            fail_fast: false,
            max_depth: 2,
            max_samples: 64,
            deadline_ms: None,
            schema_name: None,
        }
    }
}

pub enum Input<'a> {
    /// Raw model text; goes through intake (docs/03 §1).
    Text(&'a str),
    /// Already-parsed JSON; intake is skipped.
    Value(Value),
}

pub async fn verify(
    schema: &Node,
    input: Input<'_>,
    env: &Value,
    options: &Options,
    runtime: &Runtime<'_>,
) -> Report {
    let started_at = Instant::now();
    let started = now_rfc3339();

    let mut notices = Vec::new();
    let value = match input {
        Input::Value(value) => value,
        Input::Text(text) => match intake::intake(text, options.mode) {
            Ok(outcome) => {
                notices.extend(outcome.notices);
                outcome.value
            }
            Err(delta) => {
                return unrecoverable(options, *delta, notices, started, started_at);
            }
        },
    };

    let shared = engine::Shared {
        samples_left: AtomicU32::new(options.max_samples),
        usage: Mutex::new(Usage::default()),
        deadline: options
            .deadline_ms
            .map(|ms| Instant::now() + Duration::from_millis(ms)),
    };
    let sinks = engine::Sinks::default();
    let stop = AtomicBool::new(false);
    let eng = engine::Engine {
        reg: runtime.registry,
        cache: runtime.cache,
        settings: runtime.settings,
        monitor: runtime.monitor,
        opts: options,
        env,
        root: &value,
        shared: &shared,
        sinks: &sinks,
        stop: &stop,
    };

    let root = eng
        .verify_node(schema, &value, Path::root(), options.max_depth)
        .await;

    let mut deltas = Vec::new();
    root.collect_deltas(&mut deltas);
    // Stable sort: kind order (structure < constraint < semantic), then
    // document order — the ordering contract of docs/04.
    deltas.sort_by_key(|delta| delta.kind);

    notices.extend(sinks.notices.into_inner().unwrap());
    let errors = sinks.errors.into_inner().unwrap();
    let extensions = sinks.fingerprints.into_inner().unwrap();
    let usage = shared.usage.into_inner().unwrap();

    Report {
        schema: options.schema_name.clone(),
        extensions,
        verdict: root.verdict,
        deltas,
        notices,
        errors,
        root,
        usage,
        timing: Timing {
            started,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
        },
    }
}

fn unrecoverable(
    options: &Options,
    delta: Delta,
    notices: Vec<Notice>,
    started: String,
    started_at: Instant,
) -> Report {
    let root = NodeResult {
        path: "$".to_string(),
        verdict: Verdict::Fail,
        checks: vec![CheckResult::failing("structure", vec![delta.clone()])],
        children: None,
    };
    Report {
        schema: options.schema_name.clone(),
        extensions: indexmap::IndexMap::new(),
        verdict: Verdict::Fail,
        deltas: vec![delta],
        notices,
        errors: Vec::new(),
        root,
        usage: Usage::default(),
        timing: Timing {
            started,
            elapsed_ms: started_at.elapsed().as_millis() as u64,
        },
    }
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
