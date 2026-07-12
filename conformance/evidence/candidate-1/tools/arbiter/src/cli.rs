//! Diagnostic command boundary for implemented arbiter invariants.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::artifact::{self, ArtifactLimits};
use crate::digest::{bytes_sha256, h, hb, Digest};
use crate::finite;
use crate::json::{jcs, parse_ijson, render_record, ParseLimits};
use crate::path::{PinnedDirectory, PinnedFile, RepoPath, Repository};
use crate::stable::StableSelta;

const CANDIDATE: &str = "conformance/evidence/candidate-1";
const ARTIFACT_LIMIT: usize = 1_048_576;
const FIXTURE_ARTIFACT_LIMITS: ArtifactLimits = ArtifactLimits::new(301, 2, 5, 10);
const SCHEMA_FILES: [&str; 15] = [
    "artifact-set.schema.json",
    "case.schema.json",
    "completion-ledger.schema.json",
    "finite-result.schema.json",
    "finite-world.schema.json",
    "jcs-vectors.schema.json",
    "kit-index.schema.json",
    "manifest-input.schema.json",
    "manifest.schema.json",
    "oracle.schema.json",
    "prediction-ledger.schema.json",
    "request.schema.json",
    "response.schema.json",
    "runtime-evidence.schema.json",
    "seal.schema.json",
];
const FINITE_CASES: [&str; 10] = [
    "adaptive-holdout",
    "best-of-n-proxy",
    "cascade-false-reject",
    "correlated-majority",
    "correlated-unanimity",
    "genuine-disagreement",
    "incompetent-independent",
    "optional-stop",
    "recursive-fixed-point",
    "valid-log-omission",
];

#[derive(Parser)]
#[command(name = "selta-candidate-1-arbiter", disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Admit every conformance schema and execute the public JCS vectors.
    CheckFoundation {
        #[arg(long)]
        repository: PathBuf,
    },
    /// Admit and exactly evaluate all ten retained finite-world pairs.
    CheckFiniteCorpus {
        #[arg(long)]
        repository: PathBuf,
    },
    /// Validate a retained generic artifact set without constructing a new closure.
    ArtifactSet {
        #[command(subcommand)]
        command: ArtifactSetCommand,
    },
}

#[derive(Subcommand)]
enum ArtifactSetCommand {
    /// Admit one exact record and verify every member beneath its pinned root.
    Admit {
        #[arg(long)]
        repository: PathBuf,
        /// Repository-relative set root, or `.` for the repository root.
        #[arg(long)]
        root: String,
        /// Repository-relative retained artifact-set record.
        #[arg(long)]
        record: String,
        #[arg(long, default_value_t = 67_108_864)]
        max_record_bytes: usize,
        #[arg(long, default_value_t = 250_000)]
        max_artifacts: usize,
        #[arg(long, default_value_t = 17_179_869_184)]
        max_file_bytes: u64,
        #[arg(long, default_value_t = 1_099_511_627_776)]
        max_total_file_bytes: u64,
    },
}

pub(crate) fn run() -> Result<()> {
    match Cli::parse().command {
        Command::CheckFoundation { repository } => check_foundation(&repository),
        Command::CheckFiniteCorpus { repository } => check_finite_corpus(&repository),
        Command::ArtifactSet { command } => match command {
            ArtifactSetCommand::Admit {
                repository,
                root,
                record,
                max_record_bytes,
                max_artifacts,
                max_file_bytes,
                max_total_file_bytes,
            } => admit_artifact_set(
                &repository,
                &root,
                &record,
                ArtifactLimits::new(
                    max_record_bytes,
                    max_artifacts,
                    max_file_bytes,
                    max_total_file_bytes,
                ),
            ),
        },
    }
}

fn check_foundation(repository: &Path) -> Result<()> {
    let repository = Repository::open(repository)?;
    let stable = StableSelta::new();
    let schemas = format!("{CANDIDATE}/schemas").parse::<RepoPath>()?;
    let expected = SCHEMA_FILES.into_iter().collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();
    let mut folded = BTreeMap::new();
    let schema_directory = repository.open_directory(&schemas)?;
    for name in schema_directory.list(1024)? {
        if name.ends_with(b".json") {
            let name = std::str::from_utf8(&name)
                .context("matching conformance schema name is not UTF-8")?;
            if !name.is_ascii() {
                bail!("matching conformance schema name is not ASCII");
            }
            let lower = name.to_ascii_lowercase();
            if folded.insert(lower, name.to_owned()).is_some() {
                bail!("case-insensitive conformance schema path collision");
            }
            found.insert(name.to_owned());
        }
    }
    if found.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        bail!("conformance schema inventory differs from the closed 15-file set");
    }
    let mut artifact_schema = None;
    let mut vector_schema = None;
    for name in SCHEMA_FILES {
        let source = schema_directory.read_regular_file(name, ARTIFACT_LIMIT)?;
        let admitted = stable.admit_schema(&source)?;
        match name {
            "artifact-set.schema.json" => artifact_schema = Some(admitted),
            "jcs-vectors.schema.json" => vector_schema = Some(admitted),
            _ => {}
        }
        let digest = bytes_sha256(&source);
        let round_trip: Digest = digest.to_string().parse()?;
        if digest != round_trip {
            bail!("raw schema digest failed textual round trip");
        }
    }

    let vector_schema = vector_schema.context("JCS vector schema missing from closed inventory")?;
    let vectors = parse_ijson(
        &read(
            &repository,
            &format!("{CANDIDATE}/vectors/rfc8785.json"),
            ARTIFACT_LIMIT,
        )?,
        ParseLimits::SCHEMA,
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    runtime.block_on(stable.verify_value(&vector_schema, &vectors))?;
    let rows = vectors
        .get("vectors")
        .and_then(Value::as_array)
        .context("JCS vector rows missing after schema admission")?;
    let mut previous: Option<&str> = None;
    for row in rows {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .context("vector name")?;
        if previous.is_some_and(|prior| prior >= name) {
            bail!("JCS vector names are not a canonical set");
        }
        let input = row.get("input").context("vector input")?;
        let expected = decode_hex(
            row.get("expected_utf8_hex")
                .and_then(Value::as_str)
                .context("vector expected bytes")?,
        )?;
        if jcs(input)? != expected {
            bail!("JCS vector {name} differs");
        }
        if h("selta.evidence.jcs-vector-check/s2-1", input)?
            != hb("selta.evidence.jcs-vector-check/s2-1", &expected)
        {
            bail!("JCS vector {name} has inconsistent semantic identity");
        }
        previous = Some(name);
    }

    let artifact_schema = artifact_schema.context("artifact-set schema missing from inventory")?;
    check_artifact_fixture(&repository, &stable, &artifact_schema, &runtime)?;

    println!(
        "foundation pass: {} schemas, {} public JCS vectors, 1 retained artifact set",
        SCHEMA_FILES.len(),
        rows.len()
    );
    Ok(())
}

fn check_artifact_fixture(
    repository: &Repository,
    stable: &StableSelta,
    schema: &crate::stable::AdmittedSchema,
    runtime: &tokio::runtime::Runtime,
) -> Result<()> {
    const BASE: &str =
        "conformance/evidence/candidate-1/tools/arbiter/fixtures/artifact-set/minimal";
    let base_path = BASE.parse::<RepoPath>()?;
    let record_path = format!("{BASE}/artifact-set.json").parse::<RepoPath>()?;
    let base = repository.open_directory(&base_path)?;
    let root = base.open_directory(&"root".parse()?)?;
    let generated = runtime.block_on(artifact::derive(
        stable,
        schema,
        &root,
        ["nested/alias.txt".parse()?, "alpha.txt".parse()?],
        None,
        FIXTURE_ARTIFACT_LIMITS,
    ))?;
    let retained = repository.read_regular_file(&record_path, 301)?;
    if generated.bytes() != retained {
        bail!("retained artifact-set fixture differs from exact derivation");
    }
    let generated_entries = generated
        .entries()
        .map(|(path, digest)| (path.to_string(), digest))
        .collect::<Vec<_>>();
    if generated.len() != 2
        || generated_entries[0].0 != "alpha.txt"
        || generated_entries[1].0 != "nested/alias.txt"
        || generated_entries[0].1 != generated_entries[1].1
    {
        bail!("derived artifact-set fixture layout differs");
    }

    let mut record_file = repository.open_regular_file(&record_path)?;
    let admitted = runtime.block_on(artifact::admit(
        stable,
        schema,
        &root,
        &mut record_file,
        None,
        FIXTURE_ARTIFACT_LIMITS,
    ))?;
    if admitted.reference() != generated.reference() || admitted.len() != 2 {
        bail!("retained artifact-set fixture has inconsistent admission");
    }
    let expected_id: Digest =
        "sha256:369eaa591032d2497dce5af5589b6c12bc65831c2950bf105ae69a85777633ea".parse()?;
    let expected_raw: Digest =
        "sha256:c2e546dbf59f0699342d2c5a58be91e44a8a66e1c4749bf7e35d5853d356bd3c".parse()?;
    if admitted.reference().id != expected_id || admitted.reference().bytes_sha256 != expected_raw {
        bail!("retained artifact-set fixture identity differs");
    }
    let entries = admitted
        .entries()
        .map(|(path, digest)| (path.to_string(), digest))
        .collect::<Vec<_>>();
    if entries != generated_entries {
        bail!("retained artifact-set fixture layout differs");
    }
    Ok(())
}

fn check_finite_corpus(repository: &Path) -> Result<()> {
    let repository = Repository::open(repository)?;
    let stable = StableSelta::new();
    let schema_path = format!("{CANDIDATE}/schemas").parse::<RepoPath>()?;
    let schema_directory = repository.open_directory(&schema_path)?;
    let world_schema = stable.admit_schema(
        &schema_directory.read_regular_file("finite-world.schema.json", ARTIFACT_LIMIT)?,
    )?;
    let result_schema = stable.admit_schema(
        &schema_directory.read_regular_file("finite-result.schema.json", ARTIFACT_LIMIT)?,
    )?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;

    for name in FINITE_CASES {
        let input = parse_ijson(
            &read(
                &repository,
                &format!("{CANDIDATE}/finite/{name}.input.json"),
                ARTIFACT_LIMIT,
            )?,
            ParseLimits::SCHEMA,
        )?;
        let expected = parse_ijson(
            &read(
                &repository,
                &format!("{CANDIDATE}/finite/{name}.expected.json"),
                ARTIFACT_LIMIT,
            )?,
            ParseLimits::SCHEMA,
        )?;
        runtime.block_on(stable.verify_value(&world_schema, &input))?;
        runtime.block_on(stable.verify_value(&result_schema, &expected))?;
        let program = finite::admit(input)?;
        let actual = finite::evaluate(&program)?;
        let actual_value = serde_json::to_value(&actual)?;
        runtime.block_on(stable.verify_value(&result_schema, &actual_value))?;
        finite::compare_result(&actual, &expected)
            .with_context(|| format!("finite corpus case {name}"))?;
    }

    println!("finite corpus pass: {} exact pairs", FINITE_CASES.len());
    Ok(())
}

fn admit_artifact_set(
    repository_path: &Path,
    root_source: &str,
    record_source: &str,
    limits: ArtifactLimits,
) -> Result<()> {
    let repository = Repository::open(repository_path)?;
    let stable = StableSelta::new();
    let schema = stable.admit_schema(&read(
        &repository,
        &format!("{CANDIDATE}/schemas/artifact-set.schema.json"),
        ARTIFACT_LIMIT,
    )?)?;
    let record_path = record_source.parse::<RepoPath>()?;
    let (root, record_relative) = if root_source == "." {
        (repository.pinned_root()?, Some(record_path.clone()))
    } else {
        let root_path = root_source.parse::<RepoPath>()?;
        let relative = record_relative_to_root(&root_path, &record_path)?;
        (repository.open_directory(&root_path)?, relative)
    };
    let mut record_file =
        open_artifact_record(&repository, &root, &record_path, record_relative.as_ref())?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let admitted = runtime.block_on(artifact::admit(
        &stable,
        &schema,
        &root,
        &mut record_file,
        record_relative.as_ref(),
        limits,
    ))?;
    let output = render_record(&serde_json::to_value(admitted.reference())?)?;
    io::stdout()
        .lock()
        .write_all(&output)
        .context("cannot write artifact-set admission result")
}

fn record_relative_to_root(root: &RepoPath, record: &RepoPath) -> Result<Option<RepoPath>> {
    let prefix = format!("{root}/");
    if let Some(relative) = record.as_str().strip_prefix(&prefix) {
        return relative.parse().map(Some);
    }
    if record
        .as_str()
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        bail!("record and set-root paths have an ambiguous case-folded relation");
    }
    Ok(None)
}

fn open_artifact_record(
    repository: &Repository,
    root: &PinnedDirectory,
    record: &RepoPath,
    relative: Option<&RepoPath>,
) -> Result<PinnedFile> {
    match relative {
        Some(relative) => root.open_regular_file(relative),
        None => repository.open_regular_file(record),
    }
}

fn read(repository: &Repository, relative: &str, max_bytes: usize) -> Result<Vec<u8>> {
    let path = relative.parse::<RepoPath>()?;
    repository
        .read_regular_file(&path, max_bytes)
        .with_context(|| format!("cannot read {relative}"))
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if value.len() & 1 == 1 {
        bail!("hex byte string has odd length");
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = nibble(pair[0])?;
            let low = nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => bail!("hex byte string must use lowercase hexadecimal"),
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;

    use super::*;

    #[test]
    fn lowercase_hex_decoder_is_closed() {
        assert_eq!(decode_hex("00ff10").unwrap(), [0, 255, 16]);
        assert!(decode_hex("0").is_err());
        assert!(decode_hex("FF").is_err());
    }

    #[test]
    fn record_root_relation_rejects_casefold_ambiguity() {
        assert_eq!(
            record_relative_to_root(
                &"set".parse().unwrap(),
                &"set/nested/record".parse().unwrap(),
            )
            .unwrap()
            .unwrap()
            .as_str(),
            "nested/record"
        );
        assert!(record_relative_to_root(
            &"set".parse().unwrap(),
            &"outside/record".parse().unwrap(),
        )
        .unwrap()
        .is_none());
        assert!(record_relative_to_root(
            &"Set".parse().unwrap(),
            &"set/nested/record".parse().unwrap(),
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn in_root_record_open_stays_with_the_pinned_tree() {
        use std::io::Read as _;

        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join("set")).unwrap();
        fs::write(outer.path().join("set/record"), b"original").unwrap();
        let physical = fs::canonicalize(outer.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let root_path: RepoPath = "set".parse().unwrap();
        let record_path: RepoPath = "set/record".parse().unwrap();
        let relative = record_relative_to_root(&root_path, &record_path)
            .unwrap()
            .unwrap();
        let root = repository.open_directory(&root_path).unwrap();

        fs::rename(physical.join("set"), physical.join("old-set")).unwrap();
        fs::create_dir(physical.join("set")).unwrap();
        fs::write(physical.join("set/record"), b"replacement").unwrap();
        let mut record =
            open_artifact_record(&repository, &root, &record_path, Some(&relative)).unwrap();
        let mut bytes = Vec::new();
        record.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"original");
        assert_eq!(
            repository.read_regular_file(&record_path, 11).unwrap(),
            b"replacement"
        );
    }
}
