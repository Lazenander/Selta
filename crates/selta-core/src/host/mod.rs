//! The extension contract (docs/05): what Selta knows about a verifier — a
//! declaration and a way to call it. Implementations are opaque.

pub mod builtin;
pub mod rpc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Determinism {
    Deterministic,
    Nondeterministic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Needs {
    #[serde(default)]
    pub root: bool,
    #[serde(default)]
    pub env: bool,
}

impl Needs {
    pub fn from_list(items: &[String]) -> Needs {
        Needs {
            root: items.iter().any(|i| i == "root"),
            env: items.iter().any(|i| i == "env"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionDecl {
    pub name: String,
    pub determinism: Determinism,
    pub config_schema: Option<Node>,
    pub needs: Needs,
    pub settings_schema: Option<Node>,
    pub delta_schema: Option<Node>,
}

/// One verifier execution. Configs arrive fully resolved — never `$env`
/// holes — and settings arrive already merged (server ⊕ pool) and with
/// secrets injected; hosts stay stateless (docs/05).
pub struct HostCall<'a> {
    pub ext: &'a str,
    pub config: &'a Value,
    pub settings: &'a Value,
    pub value: &'a Value,
    pub path: &'a str,
    pub root: Option<&'a Value>,
    pub env: Option<&'a Value>,
    pub depth: u32,
    pub deadline_ms: Option<u64>,
}

/// The result envelope: exactly `{ verdict, delta?, usage? }` (docs/05).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub verdict: PassFail,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<WireDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<WireUsage>,
}

impl Envelope {
    pub fn pass() -> Self {
        Envelope {
            verdict: PassFail::Pass,
            delta: None,
            usage: None,
        }
    }

    pub fn fail(delta: WireDelta) -> Self {
        Envelope {
            verdict: PassFail::Fail,
            delta: Some(delta),
            usage: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PassFail {
    Pass,
    Fail,
}

/// Hosts supply only the difference itself; the engine fills in path, kind,
/// and source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireDelta {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

impl WireDelta {
    pub fn message(message: impl Into<String>) -> Self {
        WireDelta {
            message: message.into(),
            data: None,
            expected: None,
            actual: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WireUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cost_usd: f64,
}

/// An error result is "Selta could not find out" — it never votes and never
/// becomes a delta (docs/03 §Verdicts).
#[async_trait]
pub trait ExtensionHost: Send + Sync {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String>;
}
