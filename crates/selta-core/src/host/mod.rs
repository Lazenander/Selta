//! The extension contract (docs/05): what Selta knows about a verifier — a
//! declaration and a way to call it. Implementations are opaque.

pub mod builtin;
pub mod rpc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::{Node, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Determinism {
    Deterministic,
    Nondeterministic,
}

/// Coarse effect classification used by registry profiles and admission. It is
/// separate from determinism: a repeatable process invocation is still I/O.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    Pure,
    ProcessIo,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Null,
    Bool,
    Int,
    Float,
    Str,
    Object,
    Array,
}

impl InputKind {
    pub const ALL: [InputKind; 7] = [
        InputKind::Null,
        InputKind::Bool,
        InputKind::Int,
        InputKind::Float,
        InputKind::Str,
        InputKind::Object,
        InputKind::Array,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            InputKind::Null => "null",
            InputKind::Bool => "bool",
            InputKind::Int => "int",
            InputKind::Float => "float",
            InputKind::Str => "str",
            InputKind::Object => "object",
            InputKind::Array => "array",
        }
    }

    pub fn parse(value: &str) -> Option<InputKind> {
        InputKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
    }
}

/// Closed set of Selta node kinds an extension can safely receive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDomain {
    kinds: Vec<InputKind>,
}

impl InputDomain {
    pub fn new(kinds: impl IntoIterator<Item = InputKind>) -> Result<InputDomain, String> {
        let mut kinds = kinds.into_iter().collect::<Vec<_>>();
        kinds.sort_unstable();
        kinds.dedup();
        if kinds.is_empty() {
            return Err("accepted input domain must not be empty".to_string());
        }
        Ok(InputDomain { kinds })
    }

    pub fn any() -> InputDomain {
        InputDomain {
            kinds: InputKind::ALL.to_vec(),
        }
    }

    pub fn kinds(&self) -> &[InputKind] {
        &self.kinds
    }

    pub fn accepts_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Null => self.kinds.contains(&InputKind::Null),
            Type::Bool => self.kinds.contains(&InputKind::Bool),
            Type::Int => self.kinds.contains(&InputKind::Int),
            Type::Float => self.kinds.contains(&InputKind::Float),
            Type::Str => self.kinds.contains(&InputKind::Str),
            Type::Object { .. } => self.kinds.contains(&InputKind::Object),
            Type::Array { .. } => self.kinds.contains(&InputKind::Array),
            Type::Union { variants } => {
                !variants.is_empty()
                    && variants
                        .iter()
                        .all(|variant| self.accepts_type(&variant.ty))
            }
            Type::Any => InputKind::ALL.iter().all(|kind| self.kinds.contains(kind)),
        }
    }
}

pub type ConfigPreflight = fn(&Value) -> Result<(), String>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Needs {
    #[serde(default)]
    pub root: bool,
    #[serde(default)]
    pub env: bool,
}

impl Needs {
    /// Parse the closed wire vocabulary. RPC declarations use this fallible
    /// path so typos or duplicate capability claims cannot be silently
    /// projected away.
    pub fn try_from_list(items: &[String]) -> Result<Needs, String> {
        let mut needs = Needs::default();
        for item in items {
            let slot = match item.as_str() {
                "root" => &mut needs.root,
                "env" => &mut needs.env,
                other => return Err(format!("unknown need {other:?}")),
            };
            if *slot {
                return Err(format!("duplicate need {item:?}"));
            }
            *slot = true;
        }
        Ok(needs)
    }

    /// Compatibility projection for callers that intentionally accept a
    /// permissive list. Untrusted declarations must use `try_from_list`.
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
    /// Stable identity for the verifier's observable semantics. Cache entries
    /// are never shared across different revisions.
    pub semantic_revision: String,
    /// Whether deterministic executions are referentially transparent and may
    /// therefore use the configured result cache.
    pub cacheable: bool,
    pub determinism: Determinism,
    pub effect_class: EffectClass,
    pub accepted_input: InputDomain,
    pub config_schema: Option<Node>,
    pub config_preflight: Option<ConfigPreflight>,
    pub needs: Needs,
    pub settings_schema: Option<Node>,
    pub delta_schema: Option<Node>,
}

impl ExtensionDecl {
    pub fn preflight_config(&self, config: &Value) -> Result<(), String> {
        self.config_preflight
            .map_or(Ok(()), |preflight| preflight(config))
    }
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
