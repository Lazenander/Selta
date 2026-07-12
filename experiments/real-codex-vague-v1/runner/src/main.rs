use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use selta_codex_eval::{
    build_case_prompt, build_jobs, read_cases, read_oracles_bytes, read_predictions, read_prompts,
    score, sha256_file, sha256_hex, validate_predictions, validate_response, validate_schema_file,
    AttemptRole, Case, Job, Outcome, Prediction, PromptSpec, RawArtifacts, ScoreBinding,
    SeltaContract, TokenUsage,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

/// Parent Codex capabilities forbidden in a pure text-assessment child.
///
/// This source-traced list freezes the smallest direct optional gates needed by
/// CLI 0.144.1, plus declared multi-agent intent and independent shell-snapshot
/// isolation. It does not claim that Codex exposes no core/model-selected tool
/// schemas; strict event auditing remains the no-tool enforcement invariant.
const PURE_ASSESSMENT_DISABLED_CAPABILITIES: &[&str] = &[
    "plugins",
    "apps",
    "shell_tool",
    "image_generation",
    "goals",
    "hooks",
    "personality",
    "multi_agent",
    "shell_snapshot",
];

const NEUTRAL_INSTRUCTIONS: &str = "Follow the user instruction exactly.";
const PURE_ASSESSMENT_STATIC_CONFIG_OVERRIDES: &[&str] = &[
    "web_search=\"disabled\"",
    "approval_policy=\"never\"",
    "skills.include_instructions=false",
    "skills.bundled.enabled=false",
    "orchestrator.skills.enabled=false",
    "include_environment_context=false",
    "include_permissions_instructions=false",
    "include_collaboration_mode_instructions=false",
    "tools.experimental_request_user_input.enabled=false",
    "notify=[]",
    "features.multi_agent_v2.root_agent_usage_hint_text=\"\"",
    "features.multi_agent_v2.multi_agent_mode_hint_text=\"\"",
    "features.multi_agent_v2.max_concurrent_threads_per_session=1",
];
const FORBIDDEN_ISOLATED_HOME_COMPONENTS: &[&str] =
    &["plugins", "remote_plugin_catalog", "shell_snapshots"];
const LEGACY_UNSUPPORTED_UNIQUE_ITEMS_SCHEMA_SHA256: &str =
    "dbc54011037a89adc2f002e9cf55763a88007c307a51c0f455c0f27d468fd78f";

#[derive(Parser)]
#[command(name = "selta-codex-eval")]
#[command(about = "Reproducible real-Codex evidence-assessor runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run(RunArgs),
    Validate(ValidateArgs),
    VerifyCommitment(CommitmentArgs),
}

#[derive(Args)]
struct RunArgs {
    #[arg(long, value_enum)]
    mode: RunMode,
    #[arg(long)]
    inputs: PathBuf,
    #[arg(long = "prompt", required = true, value_name = "ID=PATH")]
    prompts: Vec<String>,
    #[arg(long)]
    schema: PathBuf,
    #[arg(long)]
    selta_schema: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    model: String,
    #[arg(long, value_enum)]
    reasoning: Reasoning,
    #[arg(long)]
    seed: u64,
    #[arg(long, default_value_t = 1)]
    concurrency: usize,
    #[arg(long, default_value_t = 300)]
    timeout_seconds: u64,
    #[arg(long, default_value = "codex")]
    codex: PathBuf,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    commitment: Option<PathBuf>,
}

#[derive(Args)]
struct ValidateArgs {
    #[arg(long)]
    run: Option<PathBuf>,
    #[arg(long)]
    inputs: Option<PathBuf>,
    #[arg(long)]
    predictions: Option<PathBuf>,
    #[arg(long)]
    selta_schema: Option<PathBuf>,
    #[arg(long)]
    oracle: Option<PathBuf>,
    #[arg(long)]
    nonce: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct CommitmentArgs {
    #[arg(long)]
    commitment: PathBuf,
    #[arg(long)]
    inputs: PathBuf,
    #[arg(long)]
    oracle: PathBuf,
    #[arg(long)]
    nonce: PathBuf,
}

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Reasoning {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum RunMode {
    Development,
    Heldout,
    EngineeringSmoke,
}

impl Reasoning {
    fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Manifest {
    version: u32,
    runner_version: String,
    runner_executable: Artifact,
    dry_run: bool,
    attempt_policy: String,
    inputs: Artifact,
    response_schema: Artifact,
    selta_response_schema: Artifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    heldout_commitment: Option<Artifact>,
    prompts: Vec<PromptSpec>,
    codex: CodexManifest,
    ordering: OrderingManifest,
    jobs: Artifact,
    mode: RunMode,
    mode_contract: String,
    instruction_isolation: String,
    oracle_boundary: String,
}

#[derive(Deserialize, Serialize)]
struct CodexManifest {
    program: PathBuf,
    program_sha256: String,
    version: Option<String>,
    model: String,
    reasoning: String,
    timeout_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capability_policy: Option<CodexCapabilityPolicy>,
    command_template: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct CodexCapabilityPolicy {
    disabled: Vec<String>,
    config_overrides: Vec<String>,
    neutral_instructions: String,
    neutral_instructions_sha256: String,
}

#[derive(Deserialize, Serialize)]
struct OrderingManifest {
    algorithm: String,
    seed: u64,
    concurrency: usize,
    job_count: usize,
}

#[derive(Clone, Deserialize, Serialize)]
struct Artifact {
    path: PathBuf,
    sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    records: Option<usize>,
}

#[derive(Deserialize, Serialize)]
struct Completion {
    version: u32,
    status: String,
    scored_first_attempts: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    predictions: Option<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jobs: Option<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_index: Option<Artifact>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct RawIndex {
    version: u32,
    artifacts: Vec<RawRecord>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct RawRecord {
    path: PathBuf,
    sha256: String,
    bytes: u64,
}

#[derive(Serialize)]
struct ValidationReport {
    version: u32,
    binding: String,
    inputs_sha256: String,
    predictions_sha256: String,
    cases: usize,
    predictions: usize,
    prompts: Vec<String>,
    admitted: usize,
    operational_errors: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle_receipt: Option<ScoreBinding>,
}

struct VerifiedRun {
    manifest: Manifest,
    cases: Vec<Case>,
    predictions: Vec<Prediction>,
    contract: SeltaContract,
    inputs_sha256: String,
    predictions_sha256: String,
    inputs_path: PathBuf,
    binding: ScoreBinding,
}

#[derive(Serialize)]
struct CommitmentReport {
    valid: bool,
    construction: &'static str,
    oracle_commitment_sha256: String,
    oracle_sha256: String,
    oracle_records: usize,
    inputs_sha256: String,
    input_records: usize,
}

struct Runtime<'a> {
    cases: BTreeMap<&'a str, &'a Case>,
    prompts: BTreeMap<&'a str, &'a PromptSpec>,
    codex: &'a Path,
    schema: &'a Path,
    model: &'a str,
    reasoning: &'a str,
    timeout: Duration,
    raw_dir: &'a Path,
    contract: &'a SeltaContract,
    auth: &'a Path,
}

struct PredictionSeed<'a> {
    job: &'a Job,
    prompt_sha256: &'a str,
    started: Instant,
}

impl PredictionSeed<'_> {
    fn error(
        &self,
        class: &str,
        message: impl Into<String>,
        response_contract_valid: bool,
        usage: Option<TokenUsage>,
        raw: RawArtifacts,
    ) -> Prediction {
        Prediction {
            attempt_id: self.job.attempt_id.clone(),
            attempt_role: AttemptRole::ScoredFirstAttempt,
            job_index: self.job.index,
            case_id: self.job.case_id.clone(),
            prompt_id: self.job.prompt_id.clone(),
            prompt_sha256: self.prompt_sha256.to_owned(),
            request_sha256: self.job.request_sha256.clone(),
            outcome: Outcome::OperationalError {
                class: class.to_owned(),
                message: message.into(),
            },
            response_contract_valid,
            latency_ms: elapsed_ms(self.started),
            usage,
            raw,
        }
    }

    fn admitted(
        &self,
        response: selta_codex_eval::Evidence,
        usage: Option<TokenUsage>,
        raw: RawArtifacts,
    ) -> Prediction {
        Prediction {
            attempt_id: self.job.attempt_id.clone(),
            attempt_role: AttemptRole::ScoredFirstAttempt,
            job_index: self.job.index,
            case_id: self.job.case_id.clone(),
            prompt_id: self.job.prompt_id.clone(),
            prompt_sha256: self.prompt_sha256.to_owned(),
            request_sha256: self.job.request_sha256.clone(),
            outcome: Outcome::Admitted {
                state: response.state(),
                response,
            },
            response_contract_valid: true,
            latency_ms: elapsed_ms(self.started),
            usage,
            raw,
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Run(arguments) => run(arguments),
        Commands::Validate(arguments) => validate(arguments),
        Commands::VerifyCommitment(arguments) => verify_commitment(arguments),
    }
}

fn run(arguments: RunArgs) -> Result<()> {
    #[cfg(not(unix))]
    if !arguments.dry_run {
        bail!("real runs require Unix process-group isolation; dry-run and validation remain portable");
    }
    if !(1..=4).contains(&arguments.concurrency) {
        bail!("--concurrency must be between 1 and 4");
    }
    if arguments.timeout_seconds == 0 {
        bail!("--timeout-seconds must be positive");
    }
    if arguments.model.trim().is_empty() {
        bail!("--model must be non-empty");
    }

    let source_inputs = fs::canonicalize(&arguments.inputs)
        .with_context(|| format!("cannot resolve `{}`", arguments.inputs.display()))?;
    let source_schema = fs::canonicalize(&arguments.schema)
        .with_context(|| format!("cannot resolve `{}`", arguments.schema.display()))?;
    let source_selta_schema = fs::canonicalize(&arguments.selta_schema)
        .with_context(|| format!("cannot resolve `{}`", arguments.selta_schema.display()))?;
    validate_schema_file(&source_schema)?;
    SeltaContract::from_path(&source_selta_schema)?;
    let source_cases = read_cases(&source_inputs)?;
    let source_prompts = read_prompts(&arguments.prompts)?;
    validate_mode(arguments.mode, &source_cases, &source_prompts)?;
    let source_commitment = match arguments.mode {
        RunMode::Heldout => {
            let commitment = arguments
                .commitment
                .as_ref()
                .context("heldout run requires --commitment")?;
            verify_committed_inputs(commitment, &fs::read(&source_inputs)?)?;
            Some(fs::canonicalize(commitment)?)
        }
        RunMode::Development | RunMode::EngineeringSmoke => {
            if arguments.commitment.is_some() {
                bail!("--commitment is accepted only in heldout mode");
            }
            None
        }
    };

    fs::create_dir(&arguments.output).with_context(|| {
        format!(
            "cannot create fresh output directory `{}` (it must not already exist)",
            arguments.output.display()
        )
    })?;
    let output = fs::canonicalize(&arguments.output)
        .with_context(|| format!("cannot resolve `{}`", arguments.output.display()))?;
    let artifacts = output.join("artifacts");
    let prompt_artifacts = artifacts.join("prompts");
    fs::create_dir(&artifacts).context("cannot create artifact snapshot directory")?;
    fs::create_dir(&prompt_artifacts).context("cannot create prompt snapshot directory")?;
    let inputs = snapshot_file(&source_inputs, &artifacts.join("inputs.jsonl"))?;
    let schema = snapshot_file(&source_schema, &artifacts.join("response.schema.json"))?;
    let selta_schema = snapshot_file(&source_selta_schema, &artifacts.join("response.selta.json"))?;
    let heldout_commitment = source_commitment
        .as_ref()
        .map(|source| snapshot_file(source, &artifacts.join("holdout.commitment.json")))
        .transpose()?;
    if let Some(commitment) = &heldout_commitment {
        verify_committed_inputs(commitment, &fs::read(&inputs)?)?;
    }
    let mut prompt_arguments = Vec::with_capacity(source_prompts.len());
    for prompt in &source_prompts {
        let path = prompt_artifacts.join(format!("{}.txt", prompt.id));
        fs::write(&path, prompt.content.as_bytes())
            .with_context(|| format!("cannot snapshot prompt `{}`", prompt.id))?;
        make_readonly(&path)?;
        prompt_arguments.push(format!("{}={}", prompt.id, path.display()));
    }
    validate_schema_file(&schema)?;
    let contract = SeltaContract::from_path(&selta_schema)?;
    let cases = read_cases(&inputs)?;
    let prompts = read_prompts(&prompt_arguments)?;
    validate_mode(arguments.mode, &cases, &prompts)?;
    let jobs = build_jobs(&cases, &prompts, arguments.seed);
    let jobs_bytes = jsonl_bytes(&jobs)?;
    let jobs_path = output.join("jobs.jsonl");
    fs::write(&jobs_path, &jobs_bytes).context("cannot freeze job plan")?;
    make_readonly(&jobs_path)?;

    let codex_program = resolve_program(&arguments.codex)?;
    let codex_version = match probe_codex_version(&codex_program) {
        Ok(version) => Some(version),
        Err(_) if arguments.dry_run => None,
        Err(error) => return Err(error),
    };
    let reasoning = arguments.reasoning.as_str();
    let command_template = command_template(reasoning);
    let runner_executable = fs::canonicalize(std::env::current_exe()?)?;
    let manifest_prompts: Vec<_> = prompts
        .iter()
        .cloned()
        .map(|mut prompt| {
            prompt.path = PathBuf::from(format!("artifacts/prompts/{}.txt", prompt.id));
            prompt
        })
        .collect();
    let manifest = Manifest {
        version: 1,
        runner_version: env!("CARGO_PKG_VERSION").to_owned(),
        runner_executable: Artifact {
            path: runner_executable.clone(),
            sha256: sha256_file(&runner_executable)?,
            records: None,
        },
        dry_run: arguments.dry_run,
        attempt_policy:
            "exactly one scored first attempt per prompt-case pair; no automatic retries".to_owned(),
        inputs: Artifact {
            path: PathBuf::from("artifacts/inputs.jsonl"),
            sha256: sha256_file(&inputs)?,
            records: Some(cases.len()),
        },
        response_schema: Artifact {
            path: PathBuf::from("artifacts/response.schema.json"),
            sha256: sha256_file(&schema)?,
            records: None,
        },
        selta_response_schema: Artifact {
            path: PathBuf::from("artifacts/response.selta.json"),
            sha256: sha256_file(&selta_schema)?,
            records: None,
        },
        heldout_commitment: heldout_commitment.as_ref().map(|path| Artifact {
            path: PathBuf::from("artifacts/holdout.commitment.json"),
            sha256: sha256_file(path).expect("commitment snapshot remains readable"),
            records: None,
        }),
        prompts: manifest_prompts,
        codex: CodexManifest {
            program: codex_program.clone(),
            program_sha256: sha256_file(&codex_program)?,
            version: codex_version,
            model: arguments.model.clone(),
            reasoning: reasoning.to_owned(),
            timeout_seconds: arguments.timeout_seconds,
            capability_policy: Some(CodexCapabilityPolicy {
                disabled: PURE_ASSESSMENT_DISABLED_CAPABILITIES
                    .iter()
                    .map(|capability| (*capability).to_owned())
                    .collect(),
                config_overrides: pure_assessment_config_overrides(),
                neutral_instructions: NEUTRAL_INSTRUCTIONS.to_owned(),
                neutral_instructions_sha256: sha256_hex(NEUTRAL_INSTRUCTIONS.as_bytes()),
            }),
            command_template,
        },
        ordering: OrderingManifest {
            algorithm: "ascending sha256(seed_le || 0x00 || prompt_id || 0x00 || case_id)"
                .to_owned(),
            seed: arguments.seed,
            concurrency: arguments.concurrency,
            job_count: jobs.len(),
        },
        jobs: Artifact {
            path: PathBuf::from("jobs.jsonl"),
            sha256: sha256_hex(&jobs_bytes),
            records: Some(jobs.len()),
        },
        mode: arguments.mode,
        mode_contract: "call-shape enforcement only; no corpus-validity or annotation claim"
            .to_owned(),
        instruction_isolation: "fresh CODEX_HOME starts with only a private 0600 auth.json copy; fresh empty working directory; inherited CODEX_*/OPENAI_*/CHATGPT_* removed; --ignore-user-config --ignore-rules --strict-config; exact capability denylist and neutral-context overrides recorded in codex.capability_policy; post-run CODEX_HOME state audited; strict event audit rejects any tool use"
            .to_owned(),
        oracle_boundary: "run cannot receive an oracle; score later with validate --run --oracle"
            .to_owned(),
    };
    let manifest_path = output.join("manifest.json");
    write_pretty(&manifest_path, &manifest)?;
    make_readonly(&manifest_path)?;
    let manifest_artifact = run_artifact(&manifest_path, &output, None)?;
    let jobs_artifact = run_artifact(&jobs_path, &output, Some(jobs.len()))?;

    if arguments.dry_run {
        let completion_path = output.join("completion.json");
        write_pretty(
            &completion_path,
            &Completion {
                version: 1,
                status: "dry_run_validated".to_owned(),
                scored_first_attempts: 0,
                predictions: None,
                manifest: Some(manifest_artifact),
                jobs: Some(jobs_artifact),
                raw_index: None,
            },
        )?;
        make_readonly(&completion_path)?;
        return Ok(());
    }

    let raw_dir = output.join("raw");
    fs::create_dir(&raw_dir).context("cannot create raw artifact directory")?;
    let auth = locate_codex_auth()?;
    let runtime = Runtime {
        cases: cases.iter().map(|case| (case.id.as_str(), case)).collect(),
        prompts: prompts
            .iter()
            .map(|prompt| (prompt.id.as_str(), prompt))
            .collect(),
        codex: &codex_program,
        schema: &schema,
        model: &arguments.model,
        reasoning,
        timeout: Duration::from_secs(arguments.timeout_seconds),
        raw_dir: &raw_dir,
        contract: &contract,
        auth: &auth,
    };
    let predictions = execute_jobs(&jobs, &runtime, arguments.concurrency);
    validate_run_predictions(&jobs, &predictions, &cases, &prompts, &contract)?;
    let prediction_bytes = jsonl_bytes(&predictions)?;
    let predictions_path = output.join("predictions.jsonl");
    fs::write(&predictions_path, &prediction_bytes).context("cannot write predictions")?;
    make_readonly(&predictions_path)?;
    let raw_index_path = output.join("raw-index.json");
    write_pretty(&raw_index_path, &build_raw_index(&output)?)?;
    make_readonly(&raw_index_path)?;
    let raw_index_artifact = run_artifact(&raw_index_path, &output, None)?;
    verify_manifest_snapshots(&manifest, &output)?;

    let completion_path = output.join("completion.json");
    write_pretty(
        &completion_path,
        &Completion {
            version: 1,
            status: "complete".to_owned(),
            scored_first_attempts: predictions.len(),
            predictions: Some(Artifact {
                path: PathBuf::from("predictions.jsonl"),
                sha256: sha256_hex(&prediction_bytes),
                records: Some(predictions.len()),
            }),
            manifest: Some(manifest_artifact),
            jobs: Some(jobs_artifact),
            raw_index: Some(raw_index_artifact),
        },
    )?;
    make_readonly(&completion_path)?;
    Ok(())
}

fn validate(arguments: ValidateArgs) -> Result<()> {
    let (
        binding,
        cases,
        predictions,
        contract,
        inputs_sha256,
        predictions_sha256,
        mode,
        inputs_path,
        bundle_receipt,
    ) = if let Some(run) = arguments.run.as_ref() {
        if arguments.inputs.is_some()
            || arguments.predictions.is_some()
            || arguments.selta_schema.is_some()
        {
            bail!("--run cannot be combined with loose artifact paths");
        }
        let verified = load_verified_run(run)?;
        (
            "completed_run".to_owned(),
            verified.cases,
            verified.predictions,
            verified.contract,
            verified.inputs_sha256,
            verified.predictions_sha256,
            Some(verified.manifest.mode),
            verified.inputs_path,
            Some(verified.binding),
        )
    } else {
        if arguments.oracle.is_some() || arguments.nonce.is_some() {
            bail!("oracle scoring requires --run with a completed frozen run");
        }
        let inputs = arguments
            .inputs
            .as_ref()
            .context("loose diagnostic validation requires --inputs")?;
        let predictions_path = arguments
            .predictions
            .as_ref()
            .context("loose diagnostic validation requires --predictions")?;
        let selta_schema = arguments
            .selta_schema
            .as_ref()
            .context("loose diagnostic validation requires --selta-schema")?;
        let contract = SeltaContract::from_path(selta_schema)?;
        let cases = read_cases(inputs)?;
        let predictions = read_predictions(predictions_path)?;
        validate_predictions(&cases, &predictions, &contract)?;
        (
            "loose_diagnostic_no_oracle".to_owned(),
            cases,
            predictions,
            contract,
            sha256_file(inputs)?,
            sha256_file(predictions_path)?,
            None,
            inputs.clone(),
            None,
        )
    };

    let output = if let Some(oracle_path) = arguments.oracle.as_ref() {
        let oracle_bytes = fs::read(oracle_path)
            .with_context(|| format!("cannot read oracle `{}`", oracle_path.display()))?;
        if mode == Some(RunMode::Heldout) {
            let nonce = arguments
                .nonce
                .as_ref()
                .context("heldout scoring requires --nonce")?;
            let input_bytes = fs::read(&inputs_path)?;
            let commitment = inputs_path
                .parent()
                .context("run input snapshot has no artifact directory")?
                .join("holdout.commitment.json");
            verify_commitment_bytes(&commitment, &input_bytes, &oracle_bytes, nonce)?;
        } else if arguments.nonce.is_some() {
            bail!("--nonce is only accepted for heldout scoring");
        }
        let oracles = read_oracles_bytes(&oracle_bytes, &cases, &contract)?;
        serde_json::to_value(score(
            &cases,
            &predictions,
            &oracles,
            sha256_hex(&oracle_bytes),
            &contract,
            bundle_receipt
                .clone()
                .context("scored run omits bundle receipt")?,
        )?)?
    } else {
        if arguments.nonce.is_some() {
            bail!("--nonce requires --oracle");
        }
        let mut prompts = BTreeSet::new();
        let mut admitted = 0;
        let mut errors = BTreeMap::new();
        for prediction in &predictions {
            prompts.insert(prediction.prompt_id.clone());
            match &prediction.outcome {
                Outcome::Admitted { .. } => admitted += 1,
                Outcome::OperationalError { class, .. } => {
                    *errors.entry(class.clone()).or_insert(0) += 1;
                }
            }
        }
        serde_json::to_value(ValidationReport {
            version: 1,
            binding,
            inputs_sha256,
            predictions_sha256,
            cases: cases.len(),
            predictions: predictions.len(),
            prompts: prompts.into_iter().collect(),
            admitted,
            operational_errors: errors,
            bundle_receipt,
        })?
    };

    let bytes = serde_json::to_vec_pretty(&output)?;
    if let Some(path) = arguments.output {
        fs::write(&path, bytes)
            .with_context(|| format!("cannot write validation output `{}`", path.display()))?;
    } else {
        println!("{}", String::from_utf8(bytes).expect("JSON is UTF-8"));
    }
    Ok(())
}

fn verify_commitment(arguments: CommitmentArgs) -> Result<()> {
    let report = verify_commitment_files(
        &arguments.commitment,
        &arguments.inputs,
        &arguments.oracle,
        &arguments.nonce,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn verify_committed_inputs(commitment_path: &Path, inputs: &[u8]) -> Result<()> {
    let commitment: Value = serde_json::from_slice(&fs::read(commitment_path)?)
        .context("heldout commitment is not JSON")?;
    if commitment.get("version").and_then(Value::as_u64) != Some(1) {
        bail!("unsupported heldout commitment version");
    }
    check_committed_number(&commitment, "/inputs/byte_length", inputs.len())?;
    check_committed_number(
        &commitment,
        "/inputs/record_count",
        count_jsonl_records(inputs, "heldout inputs")?,
    )?;
    if commitment.pointer("/inputs/sha256").and_then(Value::as_str)
        != Some(sha256_hex(inputs).as_str())
    {
        bail!("heldout inputs do not satisfy the frozen commitment");
    }
    Ok(())
}

fn verify_commitment_files(
    commitment_path: &Path,
    inputs_path: &Path,
    oracle_path: &Path,
    nonce_path: &Path,
) -> Result<CommitmentReport> {
    let oracle = fs::read(oracle_path)
        .with_context(|| format!("cannot read oracle `{}`", oracle_path.display()))?;
    let inputs = fs::read(inputs_path)
        .with_context(|| format!("cannot read inputs `{}`", inputs_path.display()))?;
    verify_commitment_bytes(commitment_path, &inputs, &oracle, nonce_path)
}

fn verify_commitment_bytes(
    commitment_path: &Path,
    inputs: &[u8],
    oracle: &[u8],
    nonce_path: &Path,
) -> Result<CommitmentReport> {
    const CONSTRUCTION: &str = "sha256(hex_decode(nonce) || 0x00 || exact_oracle_bytes)";

    let commitment: Value = serde_json::from_slice(
        &fs::read(commitment_path)
            .with_context(|| format!("cannot read commitment `{}`", commitment_path.display()))?,
    )
    .context("commitment is not JSON")?;
    if commitment.get("version").and_then(Value::as_u64) != Some(1)
        || commitment.get("construction").and_then(Value::as_str) != Some(CONSTRUCTION)
    {
        bail!("unsupported commitment format");
    }

    let nonce_text = fs::read_to_string(nonce_path)
        .with_context(|| format!("cannot read nonce `{}`", nonce_path.display()))?;
    let nonce = decode_lower_hex(nonce_text.trim())?;
    let expected_nonce_length = commitment
        .pointer("/nonce/decoded_byte_length")
        .and_then(Value::as_u64)
        .context("commitment omits nonce length")?;
    if nonce.len() as u64 != expected_nonce_length {
        bail!("nonce length does not match the commitment");
    }

    let mut committed_bytes = Vec::with_capacity(nonce.len() + 1 + oracle.len());
    committed_bytes.extend_from_slice(&nonce);
    committed_bytes.push(0);
    committed_bytes.extend_from_slice(oracle);
    let actual_commitment = sha256_hex(&committed_bytes);
    let expected_commitment = commitment
        .pointer("/oracle/commitment_sha256")
        .and_then(Value::as_str)
        .context("commitment omits oracle digest")?;
    if actual_commitment != expected_commitment {
        bail!("oracle and nonce do not satisfy the commitment");
    }

    let oracle_records = count_jsonl_records(oracle, "oracle")?;
    let input_records = count_jsonl_records(inputs, "inputs")?;
    check_committed_number(&commitment, "/oracle/byte_length", oracle.len())?;
    check_committed_number(&commitment, "/oracle/record_count", oracle_records)?;
    check_committed_number(&commitment, "/inputs/byte_length", inputs.len())?;
    check_committed_number(&commitment, "/inputs/record_count", input_records)?;
    let inputs_sha256 = sha256_hex(inputs);
    if commitment.pointer("/inputs/sha256").and_then(Value::as_str) != Some(inputs_sha256.as_str())
    {
        bail!("input digest does not match the commitment");
    }

    Ok(CommitmentReport {
        valid: true,
        construction: CONSTRUCTION,
        oracle_commitment_sha256: actual_commitment,
        oracle_sha256: sha256_hex(oracle),
        oracle_records,
        inputs_sha256,
        input_records,
    })
}

fn decode_lower_hex(value: &str) -> Result<Vec<u8>> {
    if value.is_empty()
        || !value.len().is_multiple_of(2)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("nonce must be non-empty lowercase hexadecimal");
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("validated ASCII");
            u8::from_str_radix(text, 16).context("invalid nonce hexadecimal")
        })
        .collect()
}

fn count_jsonl_records(bytes: &[u8], name: &str) -> Result<usize> {
    let content = std::str::from_utf8(bytes).with_context(|| format!("{name} is not UTF-8"))?;
    let mut records = 0;
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(line)
            .with_context(|| format!("{name} line {} is not JSON", index + 1))?;
        records += 1;
    }
    Ok(records)
}

fn check_committed_number(commitment: &Value, pointer: &str, actual: usize) -> Result<()> {
    let expected = commitment
        .pointer(pointer)
        .and_then(Value::as_u64)
        .with_context(|| format!("commitment omits `{pointer}`"))?;
    if actual as u64 != expected {
        bail!("artifact does not match committed `{pointer}`");
    }
    Ok(())
}

fn validate_run_predictions(
    jobs: &[Job],
    predictions: &[Prediction],
    cases: &[Case],
    prompts: &[PromptSpec],
    contract: &SeltaContract,
) -> Result<()> {
    validate_predictions(cases, predictions, contract)?;
    if jobs.len() != predictions.len() {
        bail!(
            "frozen plan has {} jobs, but execution returned {} predictions",
            jobs.len(),
            predictions.len()
        );
    }
    let prompt_digests: BTreeMap<_, _> = prompts
        .iter()
        .map(|prompt| (prompt.id.as_str(), prompt.sha256.as_str()))
        .collect();
    for (job, prediction) in jobs.iter().zip(predictions) {
        if prediction.attempt_id != job.attempt_id
            || prediction.attempt_role != job.attempt_role
            || prediction.job_index != job.index
            || prediction.case_id != job.case_id
            || prediction.prompt_id != job.prompt_id
            || prediction.request_sha256 != job.request_sha256
            || prompt_digests.get(job.prompt_id.as_str()).copied()
                != Some(prediction.prompt_sha256.as_str())
        {
            bail!(
                "prediction at job {} does not match its frozen scored-first-attempt identity",
                job.index
            );
        }
    }
    Ok(())
}

fn validate_mode(mode: RunMode, cases: &[Case], prompts: &[PromptSpec]) -> Result<()> {
    match mode {
        RunMode::Development => {
            if cases.len() != 24 || !(1..=2).contains(&prompts.len()) {
                bail!("development mode requires 24 cases and one or two prompts");
            }
        }
        RunMode::Heldout => {
            if cases.len() != 24 || !(1..=2).contains(&prompts.len()) {
                bail!("heldout mode requires 24 cases and one or two prompts");
            }
            if !prompts.iter().any(|prompt| prompt.id == "p0") {
                bail!("heldout mode requires the baseline prompt ID `p0`");
            }
        }
        RunMode::EngineeringSmoke => {
            if prompts.len() != 1 || cases.len() > 4 {
                bail!("engineering-smoke mode requires one prompt and at most four cases");
            }
        }
    }
    Ok(())
}

fn snapshot_file(source: &Path, destination: &Path) -> Result<PathBuf> {
    fs::write(destination, fs::read(source)?).with_context(|| {
        format!(
            "cannot snapshot `{}` to `{}`",
            source.display(),
            destination.display()
        )
    })?;
    make_readonly(destination)?;
    fs::canonicalize(destination)
        .with_context(|| format!("cannot resolve snapshot `{}`", destination.display()))
}

fn make_readonly(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("cannot make `{}` read-only", path.display()))
}

fn run_artifact(path: &Path, run: &Path, records: Option<usize>) -> Result<Artifact> {
    let absolute = fs::canonicalize(path)?;
    let relative = absolute
        .strip_prefix(fs::canonicalize(run)?)
        .context("run artifact escaped the run directory")?
        .to_owned();
    Ok(Artifact {
        path: relative,
        sha256: sha256_file(path)?,
        records,
    })
}

fn resolve_program(program: &Path) -> Result<PathBuf> {
    if program.components().count() > 1 {
        return fs::canonicalize(program)
            .with_context(|| format!("cannot resolve executable `{}`", program.display()));
    }
    let name = program.as_os_str();
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .map(fs::canonicalize)
        .transpose()?
        .with_context(|| format!("cannot find executable `{}` on PATH", program.display()))
}

fn locate_codex_auth() -> Result<PathBuf> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .context("cannot locate Codex home for authentication")?;
    fs::canonicalize(home.join("auth.json"))
        .context("Codex auth.json is required for a real experiment run")
}

fn verify_manifest_snapshots(manifest: &Manifest, run: &Path) -> Result<()> {
    verify_artifact(&manifest.inputs, &run.join("artifacts/inputs.jsonl"), run)?;
    verify_artifact(
        &manifest.response_schema,
        &run.join("artifacts/response.schema.json"),
        run,
    )?;
    verify_artifact(
        &manifest.selta_response_schema,
        &run.join("artifacts/response.selta.json"),
        run,
    )?;
    match (&manifest.mode, &manifest.heldout_commitment) {
        (RunMode::Heldout, Some(commitment)) => verify_artifact(
            commitment,
            &run.join("artifacts/holdout.commitment.json"),
            run,
        )?,
        (RunMode::Heldout, None) => bail!("heldout manifest omits its input commitment"),
        (_, Some(_)) => bail!("non-heldout manifest carries a heldout commitment"),
        (_, None) => {}
    }
    verify_artifact(&manifest.jobs, &run.join("jobs.jsonl"), run)?;
    let mut ids = BTreeSet::new();
    for prompt in &manifest.prompts {
        if !ids.insert(&prompt.id) {
            bail!("manifest contains duplicate prompt ID `{}`", prompt.id);
        }
        verify_artifact(
            &Artifact {
                path: prompt.path.clone(),
                sha256: prompt.sha256.clone(),
                records: None,
            },
            &run.join(format!("artifacts/prompts/{}.txt", prompt.id)),
            run,
        )?;
    }
    Ok(())
}

fn validate_frozen_response_schema(manifest: &Manifest, path: &Path) -> Result<()> {
    match validate_schema_file(path) {
        Ok(()) => Ok(()),
        Err(error)
            if manifest.mode == RunMode::EngineeringSmoke
                && manifest.response_schema.sha256
                    == LEGACY_UNSUPPORTED_UNIQUE_ITEMS_SCHEMA_SHA256 =>
        {
            // Smoke 001/002 predate the API-compatibility correction. Live
            // runs still accept only the current canonical transport schema.
            if sha256_file(path)? == LEGACY_UNSUPPORTED_UNIQUE_ITEMS_SCHEMA_SHA256 {
                Ok(())
            } else {
                Err(error)
            }
        }
        Err(error) => Err(error),
    }
}

fn verify_artifact(recorded: &Artifact, expected_path: &Path, run: &Path) -> Result<()> {
    if recorded.path.is_absolute()
        || recorded
            .path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("run-owned artifact path must be safe and relative");
    }
    let expected = fs::canonicalize(expected_path)
        .with_context(|| format!("missing frozen artifact `{}`", expected_path.display()))?;
    let recorded_path = fs::canonicalize(run.join(&recorded.path))
        .with_context(|| format!("missing recorded artifact `{}`", recorded.path.display()))?;
    if expected != recorded_path {
        bail!("recorded artifact path does not match the frozen run layout");
    }
    if sha256_file(&expected)? != recorded.sha256 {
        bail!("frozen artifact `{}` failed its digest", expected.display());
    }
    if let Some(records) = recorded.records {
        if count_jsonl_records(&fs::read(&expected)?, "frozen artifact")? != records {
            bail!(
                "frozen artifact `{}` failed its record count",
                expected.display()
            );
        }
    }
    Ok(())
}

fn build_raw_index(run: &Path) -> Result<RawIndex> {
    let raw = run.join("raw");
    let mut paths = fs::read_dir(&raw)
        .with_context(|| format!("cannot read raw artifact directory `{}`", raw.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    let mut artifacts = Vec::new();
    for path in paths {
        if !path.is_file() {
            bail!("raw artifact directory may contain only files");
        }
        let relative = path
            .strip_prefix(run)
            .context("raw artifact escaped run directory")?
            .to_owned();
        artifacts.push(RawRecord {
            path: relative,
            sha256: sha256_file(&path)?,
            bytes: fs::metadata(&path)?.len(),
        });
    }
    Ok(RawIndex {
        version: 1,
        artifacts,
    })
}

fn load_verified_run(path: &Path) -> Result<VerifiedRun> {
    let run = fs::canonicalize(path)
        .with_context(|| format!("cannot resolve run directory `{}`", path.display()))?;
    let completion_path = run.join("completion.json");
    let completion: Completion = read_json(&completion_path)?;
    if completion.version != 1 || completion.status != "complete" {
        bail!("oracle scoring requires a completed run");
    }
    let manifest_record = completion
        .manifest
        .as_ref()
        .context("completion omits manifest binding")?;
    let jobs_record = completion
        .jobs
        .as_ref()
        .context("completion omits jobs binding")?;
    let predictions_record = completion
        .predictions
        .as_ref()
        .context("completion omits predictions binding")?;
    let raw_index_record = completion
        .raw_index
        .as_ref()
        .context("completion omits raw-artifact binding")?;
    verify_artifact(manifest_record, &run.join("manifest.json"), &run)?;
    verify_artifact(jobs_record, &run.join("jobs.jsonl"), &run)?;
    verify_artifact(predictions_record, &run.join("predictions.jsonl"), &run)?;
    verify_artifact(raw_index_record, &run.join("raw-index.json"), &run)?;
    let recorded_raw: RawIndex = read_json(&run.join("raw-index.json"))?;
    if recorded_raw != build_raw_index(&run)? {
        bail!("raw artifact index does not match the completed run");
    }

    let manifest: Manifest = read_json(&run.join("manifest.json"))?;
    if manifest.version != 1 || manifest.dry_run {
        bail!("manifest is not a completed execution manifest");
    }
    verify_manifest_snapshots(&manifest, &run)?;
    if manifest.jobs.sha256 != jobs_record.sha256 || manifest.jobs.records != jobs_record.records {
        bail!("completion jobs binding disagrees with manifest");
    }
    let inputs_path = run.join("artifacts/inputs.jsonl");
    if manifest.mode == RunMode::Heldout {
        verify_committed_inputs(
            &run.join("artifacts/holdout.commitment.json"),
            &fs::read(&inputs_path)?,
        )?;
    }
    let cases = read_cases(&inputs_path)?;
    let contract = SeltaContract::from_path(&run.join("artifacts/response.selta.json"))?;
    validate_frozen_response_schema(&manifest, &run.join("artifacts/response.schema.json"))?;
    let prompt_arguments: Vec<_> = manifest
        .prompts
        .iter()
        .map(|prompt| format!("{}={}", prompt.id, run.join(&prompt.path).display()))
        .collect();
    let prompts = read_prompts(&prompt_arguments)?;
    for (recorded, loaded) in manifest.prompts.iter().zip(&prompts) {
        if recorded.id != loaded.id
            || recorded.sha256 != loaded.sha256
            || recorded.words != loaded.words
        {
            bail!("prompt snapshot does not match its manifest record");
        }
    }
    validate_mode(manifest.mode, &cases, &prompts)?;
    let jobs: Vec<Job> = read_jsonl_file(&run.join("jobs.jsonl"), "job")?;
    let expected_jobs = build_jobs(&cases, &prompts, manifest.ordering.seed);
    if jobs != expected_jobs || jobs.len() != manifest.ordering.job_count {
        bail!("frozen jobs do not reconstruct from manifest snapshots");
    }
    let predictions_path = run.join("predictions.jsonl");
    let predictions = read_predictions(&predictions_path)?;
    if predictions.len() != completion.scored_first_attempts {
        bail!("completion scored-attempt count does not match predictions");
    }
    validate_run_predictions(&jobs, &predictions, &cases, &prompts, &contract)?;
    let case_by_id: BTreeMap<_, _> = cases.iter().map(|case| (&case.id, case)).collect();
    let indexed_paths: BTreeSet<_> = recorded_raw
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_path())
        .collect();
    for prediction in &predictions {
        if let Outcome::Admitted { response, .. } = &prediction.outcome {
            let events = PathBuf::from(
                prediction
                    .raw
                    .events
                    .as_ref()
                    .context("admitted prediction omits raw event path")?,
            );
            let stderr = PathBuf::from(
                prediction
                    .raw
                    .stderr
                    .as_ref()
                    .context("admitted prediction omits raw stderr path")?,
            );
            let response_path = PathBuf::from(
                prediction
                    .raw
                    .response
                    .as_ref()
                    .context("admitted prediction omits raw response path")?,
            );
            let expected_events = format!("raw/{}.events.jsonl", prediction.attempt_id);
            let expected_stderr = format!("raw/{}.stderr.txt", prediction.attempt_id);
            let expected_response = format!("raw/{}.response.json", prediction.attempt_id);
            if events != Path::new(&expected_events)
                || stderr != Path::new(&expected_stderr)
                || response_path != Path::new(&expected_response)
                || !indexed_paths.contains(events.as_path())
                || !indexed_paths.contains(stderr.as_path())
                || !indexed_paths.contains(response_path.as_path())
            {
                bail!("admitted prediction raw paths do not match its bound attempt");
            }
            let events = parse_events(&run.join(&events))?;
            if events.disallowed_item.is_some()
                || events.terminal_error.is_some()
                || events.usage != prediction.usage
            {
                bail!("admitted prediction event audit disagrees with its outcome or usage");
            }
            let raw_response = fs::read_to_string(run.join(&response_path))?;
            let reparsed = validate_response(
                &raw_response,
                &case_by_id[&prediction.case_id].text,
                &contract,
            )?;
            if &reparsed != response {
                bail!("prediction does not match its bound raw response");
            }
        }
    }
    Ok(VerifiedRun {
        binding: ScoreBinding {
            inputs_sha256: manifest.inputs.sha256.clone(),
            predictions_sha256: predictions_record.sha256.clone(),
            manifest_sha256: manifest_record.sha256.clone(),
            jobs_sha256: jobs_record.sha256.clone(),
            completion_sha256: sha256_file(&completion_path)?,
            raw_index_sha256: raw_index_record.sha256.clone(),
        },
        inputs_sha256: manifest.inputs.sha256.clone(),
        predictions_sha256: predictions_record.sha256.clone(),
        inputs_path,
        manifest,
        cases,
        predictions,
        contract,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path)?)
        .with_context(|| format!("invalid JSON `{}`", path.display()))
}

fn read_jsonl_file<T: serde::de::DeserializeOwned>(path: &Path, kind: &str) -> Result<Vec<T>> {
    let content = fs::read_to_string(path)?;
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("invalid {kind} at line {}", index + 1))
        })
        .collect()
}

fn execute_jobs(jobs: &[Job], runtime: &Runtime<'_>, concurrency: usize) -> Vec<Prediction> {
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..concurrency.min(jobs.len()) {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(index) else {
                    break;
                };
                let prediction = execute_job(job, runtime);
                if sender.send(prediction).is_err() {
                    break;
                }
            });
        }
        drop(sender);
        let mut predictions: Vec<_> = receiver.into_iter().collect();
        predictions.sort_by_key(|prediction| prediction.job_index);
        predictions
    })
}

fn execute_job(job: &Job, runtime: &Runtime<'_>) -> Prediction {
    let started = Instant::now();
    let case = runtime.cases[job.case_id.as_str()];
    let prompt = runtime.prompts[job.prompt_id.as_str()];
    let prediction = PredictionSeed {
        job,
        prompt_sha256: &prompt.sha256,
        started,
    };

    let events_name = format!("raw/{}.events.jsonl", job.attempt_id);
    let stderr_name = format!("raw/{}.stderr.txt", job.attempt_id);
    let response_name = format!("raw/{}.response.json", job.attempt_id);
    let events_path = runtime
        .raw_dir
        .join(format!("{}.events.jsonl", job.attempt_id));
    let stderr_path = runtime
        .raw_dir
        .join(format!("{}.stderr.txt", job.attempt_id));
    let response_path = runtime
        .raw_dir
        .join(format!("{}.response.json", job.attempt_id));
    let mut raw = RawArtifacts {
        events: Some(events_name),
        stderr: Some(stderr_name),
        response: Some(response_name),
    };

    let events = match File::create(&events_path) {
        Ok(file) => file,
        Err(error) => {
            return prediction.error(
                "harness_io",
                format!("cannot create event log: {error}"),
                false,
                None,
                raw,
            )
        }
    };
    let stderr = match File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            return prediction.error(
                "harness_io",
                format!("cannot create stderr log: {error}"),
                false,
                None,
                raw,
            )
        }
    };
    let scratch = match tempfile::Builder::new()
        .prefix("selta-codex-eval-")
        .tempdir()
    {
        Ok(directory) => directory,
        Err(error) => {
            return prediction.error(
                "harness_io",
                format!("cannot create empty working directory: {error}"),
                false,
                None,
                raw,
            )
        }
    };
    let isolated_home = match tempfile::Builder::new()
        .prefix("selta-codex-home-")
        .tempdir()
    {
        Ok(directory) => directory,
        Err(error) => {
            return prediction.error(
                "harness_io",
                format!("cannot create isolated Codex home: {error}"),
                false,
                None,
                raw,
            )
        }
    };
    let isolated_auth = isolated_home.path().join("auth.json");
    if let Err(error) = fs::copy(runtime.auth, &isolated_auth)
        .and_then(|_| make_private_auth_writable(&isolated_auth))
    {
        return prediction.error(
            "harness_io",
            format!("cannot populate isolated Codex authentication: {error}"),
            false,
            None,
            raw,
        );
    }

    let mut command = Command::new(runtime.codex);
    for (name, _) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if name_text.starts_with("CODEX_")
            || name_text.starts_with("OPENAI_")
            || name_text.starts_with("CHATGPT_")
        {
            command.env_remove(name);
        }
    }
    command
        .arg("exec")
        .arg("--ephemeral")
        .arg("--ignore-user-config")
        .arg("--ignore-rules")
        .arg("--strict-config")
        .arg("--skip-git-repo-check")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--color")
        .arg("never")
        .arg("-C")
        .arg(scratch.path())
        .arg("--output-schema")
        .arg(runtime.schema)
        .arg("--json")
        .arg("--model")
        .arg(runtime.model)
        .arg("--config")
        .arg(format!("model_reasoning_effort=\"{}\"", runtime.reasoning));
    command.args(pure_assessment_capability_args());
    command
        .arg("--output-last-message")
        .arg(&response_path)
        .arg("-")
        .env("CODEX_HOME", isolated_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::from(events))
        .stderr(Stdio::from(stderr));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return prediction.error(
                "process_spawn",
                format!("cannot start Codex: {error}"),
                false,
                None,
                raw,
            )
        }
    };
    let request = build_case_prompt(&prompt.content, case);
    let Some(mut stdin) = child.stdin.take() else {
        terminate_child_tree(&mut child);
        if let Some(violation) =
            isolated_home_violation(&prediction, isolated_home.path(), None, &raw)
        {
            return violation;
        }
        return prediction.error(
            "process_stdin",
            "Codex stdin was not piped",
            false,
            None,
            raw,
        );
    };
    let (write_sender, write_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = stdin
            .write_all(request.as_bytes())
            .context("cannot write prompt");
        let _ = write_sender.send(result);
    });
    let remaining = runtime.timeout.saturating_sub(started.elapsed());
    match write_receiver.recv_timeout(remaining) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            terminate_child_tree(&mut child);
            if let Some(violation) =
                isolated_home_violation(&prediction, isolated_home.path(), None, &raw)
            {
                return violation;
            }
            return prediction.error("process_stdin", error.to_string(), false, None, raw);
        }
        Err(_) => {
            terminate_child_tree(&mut child);
            if let Some(violation) =
                isolated_home_violation(&prediction, isolated_home.path(), None, &raw)
            {
                return violation;
            }
            return prediction.error(
                "timeout",
                "Codex exceeded the timeout while receiving its prompt",
                false,
                None,
                raw,
            );
        }
    }

    let remaining = runtime.timeout.saturating_sub(started.elapsed());
    let status = match child.wait_timeout(remaining) {
        Ok(Some(status)) => status,
        Ok(None) => {
            terminate_child_tree(&mut child);
            let usage = parse_events(&events_path)
                .ok()
                .and_then(|events| events.usage);
            if let Some(violation) =
                isolated_home_violation(&prediction, isolated_home.path(), usage.clone(), &raw)
            {
                return violation;
            }
            return prediction.error(
                "timeout",
                "Codex exceeded the recorded timeout",
                false,
                usage,
                raw,
            );
        }
        Err(error) => {
            terminate_child_tree(&mut child);
            if let Some(violation) =
                isolated_home_violation(&prediction, isolated_home.path(), None, &raw)
            {
                return violation;
            }
            return prediction.error(
                "process_wait",
                format!("cannot wait for Codex: {error}"),
                false,
                None,
                raw,
            );
        }
    };

    let event_summary = parse_events(&events_path);
    let usage = event_summary
        .as_ref()
        .ok()
        .and_then(|events| events.usage.clone());
    if let Some(violation) =
        isolated_home_violation(&prediction, isolated_home.path(), usage.clone(), &raw)
    {
        return violation;
    }
    if !status.success() {
        return prediction.error(
            "process_exit",
            format!("Codex exited with {status}"),
            false,
            usage,
            raw,
        );
    }
    let events = match event_summary {
        Ok(events) => events,
        Err(error) => {
            return prediction.error("event_stream", error.to_string(), false, usage, raw)
        }
    };
    if let Some(item) = events.disallowed_item {
        return prediction.error(
            "tool_call",
            format!("Codex emitted disallowed item type `{item}`"),
            false,
            events.usage,
            raw,
        );
    }
    if let Some(message) = events.terminal_error {
        return prediction.error("codex_error", message, false, events.usage, raw);
    }

    let response = match fs::read_to_string(&response_path) {
        Ok(response) => response,
        Err(error) => {
            raw.response = None;
            return prediction.error(
                "missing_output",
                format!("cannot read final response: {error}"),
                false,
                events.usage,
                raw,
            );
        }
    };
    match validate_response(&response, &case.text, runtime.contract) {
        Ok(response) => prediction.admitted(response, events.usage, raw),
        Err(error) => prediction.error(
            error.class,
            error.message,
            error.response_contract_valid,
            events.usage,
            raw,
        ),
    }
}

fn terminate_child_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = -(child.id() as i32);
        // SAFETY: negative PID targets only the process group created for this child.
        unsafe {
            libc::kill(group, libc::SIGTERM);
        }
        std::thread::sleep(Duration::from_millis(500));
        // SAFETY: as above; ESRCH simply means the group already exited.
        unsafe {
            libc::kill(group, libc::SIGKILL);
        }
        let _ = child.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn make_private_auth_writable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)
    }
}

fn audit_isolated_codex_home(root: &Path) -> Result<()> {
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .with_context(|| format!("cannot audit isolated Codex home `{}`", root.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| {
                format!("cannot audit isolated Codex home `{}`", root.display())
            })?;
            let path = entry.path();
            let name = entry.file_name();
            if FORBIDDEN_ISOLATED_HOME_COMPONENTS
                .iter()
                .any(|forbidden| name == std::ffi::OsStr::new(forbidden))
            {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                bail!(
                    "isolated Codex home created forbidden state `{}`",
                    relative.display()
                );
            }
            if entry.file_type()?.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}

fn isolated_home_violation(
    prediction: &PredictionSeed<'_>,
    root: &Path,
    usage: Option<TokenUsage>,
    raw: &RawArtifacts,
) -> Option<Prediction> {
    audit_isolated_codex_home(root).err().map(|error| {
        prediction.error(
            "isolation_violation",
            error.to_string(),
            false,
            usage,
            raw.clone(),
        )
    })
}

#[derive(Default)]
struct EventSummary {
    usage: Option<TokenUsage>,
    disallowed_item: Option<String>,
    terminal_error: Option<String>,
}

fn parse_events(path: &Path) -> Result<EventSummary> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("cannot read event stream `{}`", path.display()))?;
    let mut summary = EventSummary::default();
    let mut count = 0;
    let mut completed = 0;
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        count += 1;
        let event: Value = serde_json::from_str(line)
            .with_context(|| format!("event line {} is not JSON", index + 1))?;
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .context("event omits string type")?;
        match event_type {
            "thread.started" | "turn.started" => {}
            "item.started" | "item.updated" | "item.completed" => {
                let item_type = event
                    .pointer("/item/type")
                    .and_then(Value::as_str)
                    .context("item event omits item.type")?;
                if !matches!(item_type, "agent_message" | "reasoning") {
                    summary.disallowed_item.get_or_insert(item_type.to_owned());
                }
            }
            "turn.completed" => {
                completed += 1;
                summary.usage = Some(
                    parse_usage(&event).context("turn.completed omits parseable token usage")?,
                );
            }
            "error" | "turn.failed" => {
                let message = event
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or(event_type);
                summary.terminal_error.get_or_insert(message.to_owned());
            }
            other => bail!("unknown Codex event type `{other}`"),
        }
    }
    if count == 0 {
        bail!("event stream is empty");
    }
    if completed != 1 {
        bail!("event stream must contain exactly one turn.completed event");
    }
    Ok(summary)
}

fn parse_usage(event: &Value) -> Option<TokenUsage> {
    let usage = event.get("usage")?;
    Some(TokenUsage {
        input_tokens: usage.get("input_tokens")?.as_u64()?,
        cached_input_tokens: usage.get("cached_input_tokens").and_then(Value::as_u64),
        output_tokens: usage.get("output_tokens")?.as_u64()?,
    })
}

fn probe_codex_version(program: &Path) -> Result<String> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .with_context(|| format!("cannot run `{}`", program.display()))?;
    if !output.status.success() {
        bail!("`{} --version` failed", program.display());
    }
    let version = String::from_utf8(output.stdout).context("Codex version is not UTF-8")?;
    let version = version.trim();
    if version.is_empty() {
        bail!("Codex reported an empty version");
    }
    Ok(version.to_owned())
}

fn command_template(reasoning: &str) -> Vec<String> {
    let mut command: Vec<String> = [
        "exec",
        "--ephemeral",
        "--ignore-user-config",
        "--ignore-rules",
        "--strict-config",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "-C",
        "<fresh-empty-directory>",
        "--output-schema",
        "<response-schema>",
        "--json",
        "--model",
        "<model>",
        "--config",
        &format!("model_reasoning_effort=\"{reasoning}\""),
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    command.extend(pure_assessment_capability_args());
    command.extend(
        ["--output-last-message", "<raw-response-path>", "-"]
            .into_iter()
            .map(str::to_owned),
    );
    command
}

fn pure_assessment_capability_args() -> Vec<String> {
    let mut arguments = Vec::new();
    for config in pure_assessment_config_overrides() {
        arguments.push("--config".to_owned());
        arguments.push(config);
    }
    for capability in PURE_ASSESSMENT_DISABLED_CAPABILITIES {
        arguments.push("--disable".to_owned());
        arguments.push((*capability).to_owned());
    }
    arguments
}

fn pure_assessment_config_overrides() -> Vec<String> {
    let mut overrides: Vec<_> = PURE_ASSESSMENT_STATIC_CONFIG_OVERRIDES
        .iter()
        .map(|config| (*config).to_owned())
        .collect();
    overrides.push(format!("instructions=\"{NEUTRAL_INSTRUCTIONS}\""));
    overrides
}

fn write_pretty(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("cannot write `{}`", path.display()))
}

fn jsonl_bytes(values: &[impl Serialize]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for value in values {
        serde_json::to_writer(&mut output, value)?;
        output.push(b'\n');
    }
    Ok(output)
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_parser_separates_usage_and_tool_calls() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("events.jsonl");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"item.completed\",\"item\":{\"type\":\"reasoning\"}}\n",
                "{\"type\":\"item.started\",\"item\":{\"type\":\"command_execution\"}}\n",
                "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":12,\"cached_input_tokens\":4,\"output_tokens\":3}}\n"
            ),
        )
        .unwrap();
        let events = parse_events(&path).unwrap();
        assert_eq!(events.disallowed_item.as_deref(), Some("command_execution"));
        assert_eq!(events.usage.unwrap().output_tokens, 3);
    }

    #[test]
    fn command_template_freezes_model_reasoning_and_assessment_surface() {
        let command = command_template("low");
        let expected = [
            "exec",
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--strict-config",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "-C",
            "<fresh-empty-directory>",
            "--output-schema",
            "<response-schema>",
            "--json",
            "--model",
            "<model>",
            "--config",
            "model_reasoning_effort=\"low\"",
            "--config",
            "web_search=\"disabled\"",
            "--config",
            "approval_policy=\"never\"",
            "--config",
            "skills.include_instructions=false",
            "--config",
            "skills.bundled.enabled=false",
            "--config",
            "orchestrator.skills.enabled=false",
            "--config",
            "include_environment_context=false",
            "--config",
            "include_permissions_instructions=false",
            "--config",
            "include_collaboration_mode_instructions=false",
            "--config",
            "tools.experimental_request_user_input.enabled=false",
            "--config",
            "notify=[]",
            "--config",
            "features.multi_agent_v2.root_agent_usage_hint_text=\"\"",
            "--config",
            "features.multi_agent_v2.multi_agent_mode_hint_text=\"\"",
            "--config",
            "features.multi_agent_v2.max_concurrent_threads_per_session=1",
            "--config",
            "instructions=\"Follow the user instruction exactly.\"",
            "--disable",
            "plugins",
            "--disable",
            "apps",
            "--disable",
            "shell_tool",
            "--disable",
            "image_generation",
            "--disable",
            "goals",
            "--disable",
            "hooks",
            "--disable",
            "personality",
            "--disable",
            "multi_agent",
            "--disable",
            "shell_snapshot",
            "--output-last-message",
            "<raw-response-path>",
            "-",
        ];
        assert_eq!(command, expected.map(str::to_owned));
    }

    #[test]
    fn baseline_prompt_keeps_byte_fidelity_and_no_repeat_invariants_lean() {
        let prompt = include_str!("../../prompts/p0.txt");
        assert_eq!(prompt.split_whitespace().count(), 88);
        assert!(prompt.contains("Copy short evidence spans byte-for-byte from the text"));
        assert!(prompt.contains("Do not repeat spans."));
    }

    #[test]
    fn legacy_smoke_bundle_without_capability_policy_remains_validatable() {
        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join("../runs/smoke-terra-low-001");
        let mut verified = load_verified_run(&run).unwrap();
        assert!(verified.manifest.codex.capability_policy.is_none());
        assert_eq!(
            verified.manifest.codex.version.as_deref(),
            Some("codex-cli 0.142.5")
        );
        assert!(matches!(
            &verified.predictions[0].outcome,
            Outcome::OperationalError { class, .. } if class == "process_exit"
        ));
        verified.manifest.mode = RunMode::Development;
        assert!(validate_frozen_response_schema(
            &verified.manifest,
            &run.join("artifacts/response.schema.json")
        )
        .is_err());
    }

    #[test]
    fn transport_failure_smoke_bundle_remains_validatable() {
        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join("../runs/smoke-terra-low-002");
        let verified = load_verified_run(&run).unwrap();
        assert!(verified.manifest.codex.capability_policy.is_some());
        assert_eq!(
            verified.manifest.codex.version.as_deref(),
            Some("codex-cli 0.144.1")
        );
        assert!(matches!(
            &verified.predictions[0].outcome,
            Outcome::OperationalError { class, .. } if class == "process_exit"
        ));
    }

    #[test]
    fn exact_membership_smoke_bundle_remains_validatable() {
        let run = Path::new(env!("CARGO_MANIFEST_DIR")).join("../runs/smoke-terra-low-003");
        let verified = load_verified_run(&run).unwrap();
        assert_eq!(verified.manifest.mode, RunMode::EngineeringSmoke);
        assert_eq!(
            verified.manifest.codex.version.as_deref(),
            Some("codex-cli 0.144.1")
        );
        assert!(verified.predictions[0].response_contract_valid);
        assert!(matches!(
            &verified.predictions[0].outcome,
            Outcome::OperationalError { class, .. } if class == "exact_quotation"
        ));
    }

    #[test]
    fn isolated_home_audit_rejects_forbidden_runtime_state() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("cache/plugins/loaded")).unwrap();
        let error = audit_isolated_codex_home(directory.path()).unwrap_err();
        assert!(error.to_string().contains("cache/plugins"));
    }

    #[test]
    fn heldout_input_preflight_binds_exact_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let commitment = directory.path().join("commitment.json");
        let inputs = b"{\"id\":\"one\"}\n";
        fs::write(
            &commitment,
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "inputs": {
                    "byte_length": inputs.len(),
                    "record_count": 1,
                    "sha256": sha256_hex(inputs)
                }
            }))
            .unwrap(),
        )
        .unwrap();
        verify_committed_inputs(&commitment, inputs).unwrap();
        let mut changed = inputs.to_vec();
        changed.push(b' ');
        assert!(verify_committed_inputs(&commitment, &changed).is_err());
    }
}
