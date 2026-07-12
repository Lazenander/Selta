//! Diagnostic command boundary for implemented arbiter invariants.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::digest::{bytes_sha256, h, hb, Digest};
use crate::finite;
use crate::json::{jcs, parse_ijson, ParseLimits};
use crate::path::{RepoPath, Repository};
use crate::stable::StableSelta;

const CANDIDATE: &str = "conformance/evidence/candidate-1";
const ARTIFACT_LIMIT: usize = 1_048_576;
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
}

pub(crate) fn run() -> Result<()> {
    match Cli::parse().command {
        Command::CheckFoundation { repository } => check_foundation(&repository),
        Command::CheckFiniteCorpus { repository } => check_finite_corpus(&repository),
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
    let mut vector_schema = None;
    for name in SCHEMA_FILES {
        let source = schema_directory.read_regular_file(name, ARTIFACT_LIMIT)?;
        let admitted = stable.admit_schema(&source)?;
        if name == "jcs-vectors.schema.json" {
            vector_schema = Some(admitted);
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

    println!(
        "foundation pass: {} schemas, {} public JCS vectors",
        SCHEMA_FILES.len(),
        rows.len()
    );
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
    use super::*;

    #[test]
    fn lowercase_hex_decoder_is_closed() {
        assert_eq!(decode_hex("00ff10").unwrap(), [0, 255, 16]);
        assert!(decode_hex("0").is_err());
        assert!(decode_hex("FF").is_err());
    }
}
