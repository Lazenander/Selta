mod score;

pub use score::{score, Evaluation, ScoreBinding};

use anyhow::{bail, Context, Result};
use selta_core::{
    verify, AdmissionPolicy, AdmissionProfile, Input, Mode, NoCache, Options, Registry, Runtime,
    Verdict,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub domain_family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarity: Option<String>,
    pub claim: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Oracle {
    pub id: String,
    pub state: EvidenceState,
    #[serde(default)]
    pub support: Vec<String>,
    #[serde(default)]
    pub refute: Vec<String>,
}

#[derive(Deserialize)]
struct OracleWire {
    id: String,
    #[serde(default, deserialize_with = "deserialize_optional_state")]
    state: Option<EvidenceState>,
    #[serde(default)]
    support: Vec<String>,
    #[serde(default)]
    refute: Vec<String>,
}

fn deserialize_optional_state<'de, D>(deserializer: D) -> Result<Option<EvidenceState>, D::Error>
where
    D: Deserializer<'de>,
{
    EvidenceState::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub support: Vec<String>,
    pub refute: Vec<String>,
}

impl Evidence {
    pub fn state(&self) -> EvidenceState {
        EvidenceState::from_presence(!self.support.is_empty(), !self.refute.is_empty())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Neither,
    SupportOnly,
    RefuteOnly,
    Both,
}

impl EvidenceState {
    pub const ALL: [Self; 4] = [
        Self::Neither,
        Self::SupportOnly,
        Self::RefuteOnly,
        Self::Both,
    ];

    pub fn from_presence(support: bool, refute: bool) -> Self {
        match (support, refute) {
            (false, false) => Self::Neither,
            (true, false) => Self::SupportOnly,
            (false, true) => Self::RefuteOnly,
            (true, true) => Self::Both,
        }
    }

    pub fn supports(self) -> bool {
        matches!(self, Self::SupportOnly | Self::Both)
    }

    pub fn refutes(self) -> bool {
        matches!(self, Self::RefuteOnly | Self::Both)
    }

    pub fn is_decisive(self) -> bool {
        matches!(self, Self::SupportOnly | Self::RefuteOnly)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neither => "neither",
            Self::SupportOnly => "support_only",
            Self::RefuteOnly => "refute_only",
            Self::Both => "both",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PromptSpec {
    pub id: String,
    pub path: PathBuf,
    pub sha256: String,
    pub words: usize,
    #[serde(skip, default)]
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Job {
    pub attempt_id: String,
    pub attempt_role: AttemptRole,
    pub index: usize,
    pub case_id: String,
    pub prompt_id: String,
    pub request_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptRole {
    ScoredFirstAttempt,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Prediction {
    pub attempt_id: String,
    pub attempt_role: AttemptRole,
    pub job_index: usize,
    pub case_id: String,
    pub prompt_id: String,
    pub prompt_sha256: String,
    pub request_sha256: String,
    pub outcome: Outcome,
    pub response_contract_valid: bool,
    pub latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    pub raw: RawArtifacts,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Admitted {
        state: EvidenceState,
        response: Evidence,
    },
    OperationalError {
        class: String,
        message: String,
    },
}

impl Outcome {
    pub fn state(&self) -> Option<EvidenceState> {
        match self {
            Self::Admitted { state, .. } => Some(*state),
            Self::OperationalError { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RawArtifacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub class: &'static str,
    pub message: String,
    pub response_contract_valid: bool,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// One strictly admitted Selta-1 response contract evaluated only by pure
/// in-process builtins.
pub struct SeltaContract {
    registry: Registry,
    schema: selta_core::Node,
    cache: NoCache,
}

impl SeltaContract {
    pub fn from_source(source: &[u8]) -> Result<Self> {
        let registry = Registry::with_pure_builtins();
        let schema = registry
            .admit_source_at(
                source,
                &AdmissionPolicy::pure_only(),
                AdmissionProfile::Selta1,
            )
            .map_err(|issues| {
                let detail = issues
                    .iter()
                    .map(|issue| {
                        let pointer = if issue.pointer.is_empty() {
                            "/"
                        } else {
                            &issue.pointer
                        };
                        format!("{} at {pointer}: {}", issue.code, issue.detail)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                anyhow::anyhow!("Selta response schema was not admitted: {detail}")
            })?
            .into_node();
        Ok(Self {
            registry,
            schema,
            cache: NoCache,
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let source = fs::read(path)
            .with_context(|| format!("cannot read Selta schema `{}`", path.display()))?;
        let supplied: Value = serde_json::from_slice(&source)
            .with_context(|| format!("Selta schema `{}` is not JSON", path.display()))?;
        let canonical: Value =
            serde_json::from_str(include_str!("../../schemas/response.selta.json"))
                .expect("bundled canonical Selta schema is JSON");
        if supplied != canonical {
            bail!("Selta schema must equal the bundled canonical response contract");
        }
        Self::from_source(&source)
    }

    pub fn verify_raw(&self, raw: &str) -> Result<(), ValidationError> {
        self.verify(Input::Text(raw))
    }

    pub fn verify_evidence(&self, evidence: &Evidence) -> Result<(), ValidationError> {
        let value = serde_json::to_value(evidence).expect("Evidence always serializes");
        self.verify(Input::Value(value))
    }

    fn verify(&self, input: Input<'_>) -> Result<(), ValidationError> {
        let options = Options {
            mode: Mode::Strict,
            ..Options::default()
        };
        let runtime = Runtime::new(&self.registry, &self.cache);
        let report = futures::executor::block_on(verify(
            &self.schema,
            input,
            &Value::Object(Default::default()),
            &options,
            &runtime,
        ));
        match report.verdict {
            Verdict::Pass => Ok(()),
            Verdict::Fail => Err(ValidationError {
                class: "response_contract",
                message: report
                    .deltas
                    .first()
                    .map(|delta| format!("Selta rejected the response: {}", delta.message))
                    .unwrap_or_else(|| "Selta rejected the response".to_owned()),
                response_contract_valid: false,
            }),
            Verdict::Inconclusive => Err(ValidationError {
                class: "selta_error",
                message: report
                    .errors
                    .first()
                    .map(|error| format!("Selta could not verify the response: {}", error.error))
                    .unwrap_or_else(|| "Selta verification was inconclusive".to_owned()),
                response_contract_valid: false,
            }),
        }
    }
}

pub fn read_cases(path: &Path) -> Result<Vec<Case>> {
    let cases: Vec<Case> = read_jsonl(path, "input case")?;
    if cases.is_empty() {
        bail!("input corpus is empty");
    }
    let mut ids = BTreeSet::new();
    for case in &cases {
        if case.id.is_empty() || case.domain_family.is_empty() {
            bail!("case IDs and domain families must be non-empty");
        }
        if !ids.insert(&case.id) {
            bail!("duplicate case ID `{}`", case.id);
        }
    }
    Ok(cases)
}

pub fn read_oracles(path: &Path, cases: &[Case], contract: &SeltaContract) -> Result<Vec<Oracle>> {
    let bytes =
        fs::read(path).with_context(|| format!("cannot read oracle JSONL `{}`", path.display()))?;
    read_oracles_bytes(&bytes, cases, contract)
}

pub fn read_oracles_bytes(
    bytes: &[u8],
    cases: &[Case],
    contract: &SeltaContract,
) -> Result<Vec<Oracle>> {
    let content = std::str::from_utf8(bytes).context("oracle JSONL is not UTF-8")?;
    let wires: Vec<OracleWire> = content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("invalid oracle on line {}", index + 1))
        })
        .collect::<Result<_>>()?;
    let case_by_id: BTreeMap<_, _> = cases.iter().map(|case| (&case.id, case)).collect();
    let mut ids = BTreeSet::new();
    let mut oracles = Vec::with_capacity(wires.len());
    for wire in wires {
        let case = case_by_id
            .get(&wire.id)
            .with_context(|| format!("oracle references unknown case `{}`", wire.id))?;
        if !ids.insert(wire.id.clone()) {
            bail!("duplicate oracle for case `{}`", wire.id);
        }
        let evidence = Evidence {
            support: wire.support,
            refute: wire.refute,
        };
        contract
            .verify_evidence(&evidence)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("oracle for case `{}` failed Selta", wire.id))?;
        validate_evidence(&evidence, &case.text)
            .map_err(anyhow::Error::new)
            .with_context(|| format!("invalid oracle for case `{}`", wire.id))?;
        let state = evidence.state();
        if let Some(authored) = wire.state {
            if authored != state {
                bail!(
                    "oracle state for `{}` is {:?}, but its spans derive {:?}",
                    wire.id,
                    authored,
                    state
                );
            }
        }
        oracles.push(Oracle {
            id: wire.id,
            state,
            support: evidence.support,
            refute: evidence.refute,
        });
    }
    if ids.len() != cases.len() {
        let missing: Vec<_> = cases
            .iter()
            .filter(|case| !ids.contains(&case.id))
            .map(|case| case.id.as_str())
            .collect();
        bail!("oracle is missing cases: {}", missing.join(", "));
    }
    Ok(oracles)
}

pub fn read_predictions(path: &Path) -> Result<Vec<Prediction>> {
    read_jsonl(path, "prediction")
}

pub fn read_prompts(arguments: &[String]) -> Result<Vec<PromptSpec>> {
    let mut prompts = Vec::with_capacity(arguments.len());
    let mut ids = BTreeSet::new();
    for argument in arguments {
        let (id, raw_path) = argument
            .split_once('=')
            .with_context(|| format!("prompt `{argument}` must use ID=PATH"))?;
        if id.is_empty()
            || !id
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        {
            bail!("prompt ID `{id}` is not a safe opaque identifier");
        }
        if !ids.insert(id.to_owned()) {
            bail!("duplicate prompt ID `{id}`");
        }
        let path = fs::canonicalize(raw_path)
            .with_context(|| format!("cannot resolve prompt `{raw_path}`"))?;
        let bytes =
            fs::read(&path).with_context(|| format!("cannot read prompt `{}`", path.display()))?;
        let content = String::from_utf8(bytes.clone())
            .with_context(|| format!("prompt `{}` is not UTF-8", path.display()))?;
        if content.trim().is_empty() {
            bail!("prompt `{id}` is empty");
        }
        let words = content.split_whitespace().count();
        if words > 90 {
            bail!("prompt `{id}` has {words} words; the protocol limit is 90");
        }
        prompts.push(PromptSpec {
            id: id.to_owned(),
            path,
            sha256: sha256_hex(&bytes),
            words,
            content,
        });
    }
    if prompts.is_empty() {
        bail!("at least one prompt is required");
    }
    Ok(prompts)
}

pub fn validate_schema_file(path: &Path) -> Result<()> {
    let value: Value = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("cannot read schema `{}`", path.display()))?,
    )
    .with_context(|| format!("schema `{}` is not valid JSON", path.display()))?;
    let required = value
        .get("required")
        .and_then(Value::as_array)
        .context("response schema must declare required fields")?;
    let required: BTreeSet<_> = required.iter().filter_map(Value::as_str).collect();
    let properties = value
        .get("properties")
        .and_then(Value::as_object)
        .context("response schema must declare object properties")?;
    let property_names: BTreeSet<_> = properties.keys().map(String::as_str).collect();
    if value.get("type").and_then(Value::as_str) != Some("object")
        || value.get("additionalProperties").and_then(Value::as_bool) != Some(false)
        || required != BTreeSet::from(["support", "refute"])
        || property_names != BTreeSet::from(["support", "refute"])
    {
        bail!("response schema must be the closed support/refute object contract");
    }
    for polarity in ["support", "refute"] {
        let property = &properties[polarity];
        if property.get("type").and_then(Value::as_str) != Some("array")
            || property.get("maxItems").and_then(Value::as_u64) != Some(3)
            || property.pointer("/items/type").and_then(Value::as_str) != Some("string")
            || property.pointer("/items/minLength").and_then(Value::as_u64) != Some(1)
            || property.pointer("/items/maxLength").and_then(Value::as_u64) != Some(160)
        {
            bail!(
                "response schema `{polarity}` must contain up to three non-empty strings of at most 160 characters"
            );
        }
    }
    let canonical: Value = serde_json::from_str(include_str!("../../schemas/response.schema.json"))
        .expect("bundled transport schema is JSON");
    if value != canonical {
        bail!("response schema must equal the bundled canonical transport schema");
    }
    Ok(())
}

pub fn build_case_prompt(base: &str, case: &Case) -> String {
    #[derive(Serialize)]
    struct Payload<'a> {
        claim: &'a str,
        text: &'a str,
    }

    let payload = serde_json::to_string(&Payload {
        claim: &case.claim,
        text: &case.text,
    })
    .expect("string payload always serializes");
    let separator = if base.ends_with('\n') {
        "\nInput:\n"
    } else {
        "\n\nInput:\n"
    };
    format!("{base}{separator}{payload}")
}

pub fn build_jobs(cases: &[Case], prompts: &[PromptSpec], seed: u64) -> Vec<Job> {
    let mut ordered: Vec<_> = prompts
        .iter()
        .flat_map(|prompt| {
            cases.iter().map(move |case| {
                let request = build_case_prompt(&prompt.content, case);
                let mut ordering = Sha256::new();
                ordering.update(seed.to_le_bytes());
                ordering.update([0]);
                ordering.update(prompt.id.as_bytes());
                ordering.update([0]);
                ordering.update(case.id.as_bytes());
                (
                    ordering.finalize().to_vec(),
                    case.id.clone(),
                    prompt.id.clone(),
                    sha256_hex(request.as_bytes()),
                )
            })
        })
        .collect();
    ordered.sort_by(|left, right| (&left.0, &left.2, &left.1).cmp(&(&right.0, &right.2, &right.1)));
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, (_, case_id, prompt_id, request_sha256))| Job {
            attempt_id: format!("a{:04}", index + 1),
            attempt_role: AttemptRole::ScoredFirstAttempt,
            index,
            case_id,
            prompt_id,
            request_sha256,
        })
        .collect()
}

pub fn validate_response(
    raw: &str,
    text: &str,
    contract: &SeltaContract,
) -> Result<Evidence, ValidationError> {
    contract.verify_raw(raw)?;
    let evidence: Evidence = serde_json::from_str(raw).map_err(|error| ValidationError {
        class: "selta_projection",
        message: format!("Selta-admitted response did not project to Evidence: {error}"),
        response_contract_valid: false,
    })?;
    validate_evidence(&evidence, text)?;
    Ok(evidence)
}

pub fn validate_evidence(evidence: &Evidence, text: &str) -> Result<(), ValidationError> {
    if evidence
        .support
        .iter()
        .any(|span| evidence.refute.contains(span))
    {
        return Err(ValidationError {
            class: "response_contract",
            message: "the same quotation must not appear on both sides".to_owned(),
            response_contract_valid: false,
        });
    }
    for (polarity, spans) in [("support", &evidence.support), ("refute", &evidence.refute)] {
        if spans.len() > 3 {
            return Err(ValidationError {
                class: "response_contract",
                message: format!("{polarity} contains more than three quotations"),
                response_contract_valid: false,
            });
        }
        let mut unique = BTreeSet::new();
        for span in spans {
            let length = span.chars().count();
            if span.is_empty() || length > 160 || !unique.insert(span) {
                return Err(ValidationError {
                    class: "response_contract",
                    message: format!(
                        "{polarity} quotations must be non-empty, unique, and at most 160 characters"
                    ),
                    response_contract_valid: false,
                });
            }
            if !text.contains(span) {
                return Err(ValidationError {
                    class: "exact_quotation",
                    message: format!(
                        "{polarity} quotation does not occur exactly in the input text"
                    ),
                    response_contract_valid: true,
                });
            }
        }
    }
    Ok(())
}

pub fn validate_predictions(
    cases: &[Case],
    predictions: &[Prediction],
    contract: &SeltaContract,
) -> Result<()> {
    if predictions.is_empty() {
        bail!("prediction set is empty");
    }
    let cases: BTreeMap<_, _> = cases.iter().map(|case| (&case.id, case)).collect();
    let mut attempts = BTreeSet::new();
    let mut job_indices = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut prompt_digests = BTreeMap::new();
    let mut prompt_ids = BTreeSet::new();
    for (position, prediction) in predictions.iter().enumerate() {
        let case = cases.get(&prediction.case_id).with_context(|| {
            format!(
                "prediction references unknown case `{}`",
                prediction.case_id
            )
        })?;
        if !attempts.insert(&prediction.attempt_id) {
            bail!("duplicate attempt ID `{}`", prediction.attempt_id);
        }
        if prediction.job_index != position || !job_indices.insert(prediction.job_index) {
            bail!(
                "prediction `{}` is not in its unique frozen job position",
                prediction.attempt_id
            );
        }
        if !pairs.insert((&prediction.prompt_id, &prediction.case_id)) {
            bail!(
                "duplicate prediction for prompt `{}` and case `{}`",
                prediction.prompt_id,
                prediction.case_id
            );
        }
        prompt_ids.insert(&prediction.prompt_id);
        if !is_sha256(&prediction.prompt_sha256) || !is_sha256(&prediction.request_sha256) {
            bail!(
                "prediction `{}` has a malformed artifact digest",
                prediction.attempt_id
            );
        }
        match prompt_digests.insert(&prediction.prompt_id, &prediction.prompt_sha256) {
            Some(previous) if previous != &prediction.prompt_sha256 => {
                bail!("prompt `{}` has inconsistent digests", prediction.prompt_id)
            }
            _ => {}
        }
        match &prediction.outcome {
            Outcome::Admitted { state, response } => {
                contract
                    .verify_evidence(response)
                    .map_err(anyhow::Error::new)
                    .with_context(|| {
                        format!(
                            "admitted prediction `{}` failed Selta on readback",
                            prediction.attempt_id
                        )
                    })?;
                validate_evidence(response, &case.text)
                    .map_err(anyhow::Error::new)
                    .with_context(|| {
                        format!("invalid admitted prediction `{}`", prediction.attempt_id)
                    })?;
                if response.state() != *state {
                    bail!(
                        "prediction `{}` carries state {:?}, but its evidence derives {:?}",
                        prediction.attempt_id,
                        state,
                        response.state()
                    );
                }
                if !prediction.response_contract_valid {
                    bail!(
                        "admitted prediction `{}` is marked contract-invalid",
                        prediction.attempt_id
                    );
                }
            }
            Outcome::OperationalError { class, .. } => {
                if class.is_empty() {
                    bail!(
                        "prediction `{}` has an empty error class",
                        prediction.attempt_id
                    );
                }
            }
        }
    }
    for prompt in prompt_ids {
        for case in cases.keys() {
            if !pairs.contains(&(prompt, case)) {
                bail!(
                    "prompt `{}` is missing a prediction for case `{}`",
                    prompt,
                    case
                );
            }
        }
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    Ok(sha256_hex(&fs::read(path).with_context(|| {
        format!("cannot read `{}` for hashing", path.display())
    })?))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    output
}

fn read_jsonl<T: DeserializeOwned>(path: &Path, kind: &str) -> Result<Vec<T>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("cannot read {kind} JSONL `{}`", path.display()))?;
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line).with_context(|| {
                format!(
                    "invalid {kind} on line {} of `{}`",
                    index + 1,
                    path.display()
                )
            })
        })
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str, text: &str) -> Case {
        Case {
            id: id.into(),
            domain_family: "test".into(),
            clarity: Some("clear".into()),
            claim: "A claim".into(),
            text: text.into(),
        }
    }

    fn contract() -> SeltaContract {
        SeltaContract::from_source(include_bytes!("../../schemas/response.selta.json")).unwrap()
    }

    #[test]
    fn response_derives_all_four_states() {
        let text = "alpha contradicts beta";
        for (raw, expected) in [
            (r#"{"support":[],"refute":[]}"#, EvidenceState::Neither),
            (
                r#"{"support":["alpha"],"refute":[]}"#,
                EvidenceState::SupportOnly,
            ),
            (
                r#"{"support":[],"refute":["beta"]}"#,
                EvidenceState::RefuteOnly,
            ),
            (
                r#"{"support":["alpha"],"refute":["beta"]}"#,
                EvidenceState::Both,
            ),
        ] {
            assert_eq!(
                validate_response(raw, text, &contract()).unwrap().state(),
                expected
            );
        }
    }

    #[test]
    fn response_contract_and_exact_quote_failures_stay_distinct() {
        let contract = contract();
        let unknown = validate_response(
            r#"{"support":[],"refute":[],"state":"neither"}"#,
            "x",
            &contract,
        )
        .unwrap_err();
        assert_eq!(unknown.class, "response_contract");
        assert!(!unknown.response_contract_valid);

        let invented = validate_response(r#"{"support":["invented"],"refute":[]}"#, "x", &contract)
            .unwrap_err();
        assert_eq!(invented.class, "exact_quotation");
        assert!(invented.response_contract_valid);

        let cross_side_raw = r#"{"support":["x"],"refute":["x"]}"#;
        assert!(contract.verify_raw(cross_side_raw).is_ok());
        let cross_side_duplicate = validate_response(cross_side_raw, "x", &contract).unwrap_err();
        assert_eq!(cross_side_duplicate.class, "response_contract");

        let within_side_raw = r#"{"support":["x","x"],"refute":[]}"#;
        assert!(contract.verify_raw(within_side_raw).is_ok());
        let within_side_duplicate = validate_response(within_side_raw, "x", &contract).unwrap_err();
        assert_eq!(within_side_duplicate.class, "response_contract");
    }

    #[test]
    fn oracle_without_authored_state_derives_all_four_states() {
        let cases = vec![
            case("neither", "alpha beta"),
            case("support", "alpha beta"),
            case("refute", "alpha beta"),
            case("both", "alpha beta"),
        ];
        let bytes = concat!(
            "{\"id\":\"neither\",\"support\":[],\"refute\":[],\"metadata\":\"ignored\"}\n",
            "{\"id\":\"support\",\"support\":[\"alpha\"],\"refute\":[]}\n",
            "{\"id\":\"refute\",\"support\":[],\"refute\":[\"beta\"]}\n",
            "{\"id\":\"both\",\"support\":[\"alpha\"],\"refute\":[\"beta\"]}\n",
        );
        let oracles = read_oracles_bytes(bytes.as_bytes(), &cases, &contract()).unwrap();
        assert_eq!(
            oracles
                .iter()
                .map(|oracle| oracle.state)
                .collect::<Vec<_>>(),
            EvidenceState::ALL
        );
    }

    #[test]
    fn authored_oracle_state_must_match_derived_state() {
        let cases = vec![case("one", "alpha")];
        let error = read_oracles_bytes(
            br#"{"id":"one","state":"neither","support":["alpha"],"refute":[]}
"#,
            &cases,
            &contract(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("spans derive SupportOnly"));
    }

    #[test]
    fn explicit_null_oracle_state_is_rejected() {
        let cases = vec![case("one", "alpha")];
        let error = read_oracles_bytes(
            br#"{"id":"one","state":null,"support":["alpha"],"refute":[]}
"#,
            &cases,
            &contract(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid oracle on line 1"));
    }

    #[test]
    fn authored_development_oracle_behavior_is_unchanged() {
        let experiment = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let cases = read_cases(&experiment.join("corpus/dev.inputs.jsonl")).unwrap();
        let oracles = read_oracles(
            &experiment.join("corpus/dev.oracle.jsonl"),
            &cases,
            &contract(),
        )
        .unwrap();
        assert_eq!(oracles.len(), 24);
        assert!(oracles.iter().all(|oracle| {
            oracle.state
                == EvidenceState::from_presence(
                    !oracle.support.is_empty(),
                    !oracle.refute.is_empty(),
                )
        }));
    }

    #[test]
    fn job_order_is_seeded_and_prompt_payload_excludes_metadata() {
        let cases = vec![case("secret-id", "supplied text"), case("other", "second")];
        let prompts = vec![PromptSpec {
            id: "p0".into(),
            path: PathBuf::from("p0.txt"),
            sha256: "digest".into(),
            words: 2,
            content: "Assess carefully.".into(),
        }];
        assert_eq!(
            build_jobs(&cases, &prompts, 7),
            build_jobs(&cases, &prompts, 7)
        );
        assert_ne!(
            build_jobs(&cases, &prompts, 7),
            build_jobs(&cases, &prompts, 8)
        );

        let payload = build_case_prompt(&prompts[0].content, &cases[0]);
        assert!(payload.contains("supplied text"));
        assert!(!payload.contains("secret-id"));
        assert!(!payload.contains("domain_family"));
        assert!(!payload.contains("clarity"));
    }
}
