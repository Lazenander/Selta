//! The extension contract (docs/05): what Selta knows about a verifier — a
//! declaration and a way to call it. Implementations are opaque.

pub mod builtin;
pub mod rpc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::{Node, Type};
use crate::verdict::EvidenceState;

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

/// One evidence-preserving assessment. Support and refutation are independent:
/// the four combinations represent neither, support only, refutation only, and
/// conflict. Operational failure remains the outer `Result::Err`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessmentEnvelope {
    pub support: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refute: Option<WireDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<WireUsage>,
}

impl AssessmentEnvelope {
    pub fn neither() -> Self {
        Self {
            support: false,
            refute: None,
            usage: None,
        }
    }

    pub fn support() -> Self {
        Self {
            support: true,
            ..Self::neither()
        }
    }

    pub fn refute(delta: WireDelta) -> Self {
        Self {
            refute: Some(delta),
            ..Self::neither()
        }
    }

    pub fn both(delta: WireDelta) -> Self {
        Self {
            support: true,
            refute: Some(delta),
            usage: None,
        }
    }

    pub fn state(&self) -> EvidenceState {
        EvidenceState::from_presence(self.support, self.refute.is_some())
    }

    /// Preserve the exact legacy host statement without widening the legacy
    /// envelope. A malformed fail remains operationally unavailable.
    pub fn from_legacy(envelope: Envelope) -> Result<Self, String> {
        match envelope.verdict {
            PassFail::Pass => Ok(Self {
                support: true,
                refute: None,
                usage: envelope.usage,
            }),
            PassFail::Fail => {
                let delta = envelope
                    .delta
                    .ok_or_else(|| "fail verdict without a delta".to_string())?;
                Ok(Self {
                    support: false,
                    refute: Some(delta),
                    usage: envelope.usage,
                })
            }
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

    /// Evidence-preserving execution is additive. Existing hosts inherit an
    /// exact embedding of their binary result; native assessors override this
    /// method to express semantic abstention or conflict directly.
    async fn assess(&self, call: HostCall<'_>) -> Result<AssessmentEnvelope, String> {
        AssessmentEnvelope::from_legacy(self.verify(call).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LegacyFailHost;

    #[async_trait]
    impl ExtensionHost for LegacyFailHost {
        async fn verify(&self, _call: HostCall<'_>) -> Result<Envelope, String> {
            Ok(Envelope::fail(WireDelta::message("legacy refutation")))
        }
    }

    fn call<'a>(value: &'a Value, empty: &'a Value) -> HostCall<'a> {
        HostCall {
            ext: "judge",
            config: empty,
            settings: empty,
            value,
            path: "$",
            root: None,
            env: None,
            depth: 1,
            deadline_ms: None,
        }
    }

    #[test]
    fn assessment_constructors_cover_the_four_presence_states() {
        assert_eq!(
            AssessmentEnvelope::neither().state(),
            EvidenceState::Neither
        );
        assert_eq!(
            AssessmentEnvelope::support().state(),
            EvidenceState::SupportOnly
        );
        assert_eq!(
            AssessmentEnvelope::refute(WireDelta::message("no")).state(),
            EvidenceState::RefuteOnly
        );
        assert_eq!(
            AssessmentEnvelope::both(WireDelta::message("conflict")).state(),
            EvidenceState::Both
        );
    }

    #[test]
    fn assessment_wire_requires_support_and_rejects_unknown_fields() {
        assert!(serde_json::from_str::<AssessmentEnvelope>(r#"{}"#).is_err());
        assert!(
            serde_json::from_str::<AssessmentEnvelope>(r#"{"support":false,"surprise":true}"#)
                .is_err()
        );
        let neither: AssessmentEnvelope =
            serde_json::from_str(r#"{"support":false}"#).expect("minimal assessment");
        assert_eq!(neither.state(), EvidenceState::Neither);
    }

    #[test]
    fn legacy_embedding_preserves_usage_and_rejects_delta_less_failures() {
        let mut pass = Envelope::pass();
        pass.usage = Some(WireUsage {
            input_tokens: 4,
            output_tokens: 2,
            cost_usd: 0.25,
        });
        let assessment = AssessmentEnvelope::from_legacy(pass).expect("pass embeds");
        assert_eq!(assessment.state(), EvidenceState::SupportOnly);
        assert_eq!(assessment.usage.expect("usage").input_tokens, 4);

        let malformed = Envelope {
            verdict: PassFail::Fail,
            delta: None,
            usage: None,
        };
        assert_eq!(
            AssessmentEnvelope::from_legacy(malformed).expect_err("delta is required"),
            "fail verdict without a delta"
        );
    }

    #[tokio::test]
    async fn default_assess_calls_and_embeds_the_legacy_verifier() {
        let value = Value::String("answer".to_string());
        let empty = serde_json::json!({});
        let assessment = LegacyFailHost
            .assess(call(&value, &empty))
            .await
            .expect("legacy assess");
        assert_eq!(assessment.state(), EvidenceState::RefuteOnly);
        assert_eq!(
            assessment.refute.expect("refutation").message,
            "legacy refutation"
        );
    }
}
