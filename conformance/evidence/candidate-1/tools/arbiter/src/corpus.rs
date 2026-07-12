//! Closed recursive inventory and local admission of the retained S2 corpus.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::inventory::{inventory_recursive, InventoriedFile, InventoryLimits};
use crate::json::{parse_ijson, ParseLimits};
use crate::path::{RepoPath, Repository};
use crate::stable::{AdmittedSchema, StableSelta};

pub(crate) const SCHEMA_FILES: [&str; 15] = [
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

const CASE_ROOT: &str = "conformance/evidence/candidate-1/cases";
const ORACLE_ROOT: &str = "conformance/evidence/candidate-1/oracles";
const SCHEMA_ROOT: &str = "conformance/evidence/candidate-1/schemas";
const EXPECTED_RECORDS: usize = 163;
const INVENTORY_LIMITS: InventoryLimits = InventoryLimits {
    max_depth: 16,
    max_entries_per_directory: 1_024,
    max_total_entries: 4_096,
    max_selected_files: 1_024,
    max_file_bytes: 1_048_576,
    max_selected_bytes: 16_777_216,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CorpusSummary {
    pub(crate) schemas: usize,
    pub(crate) cases: usize,
    pub(crate) oracles: usize,
    pub(crate) categories: BTreeMap<String, usize>,
    pub(crate) operations: BTreeMap<String, usize>,
    pub(crate) oracle_kinds: BTreeMap<String, usize>,
}

impl CorpusSummary {
    pub(crate) fn category_count(&self, category: &str) -> usize {
        self.categories.get(category).copied().unwrap_or_default()
    }

    pub(crate) fn operation_count(&self, kind: &str) -> usize {
        self.operations.get(kind).copied().unwrap_or_default()
    }

    pub(crate) fn oracle_kind_count(&self, kind: &str) -> usize {
        self.oracle_kinds.get(kind).copied().unwrap_or_default()
    }
}

/// Inventory and locally admit the closed case/oracle/schema trees.
///
/// This verifies only filesystem closure, stable Selta admissions, pairing,
/// IDs, and the operation/expected-shape relation. It neither follows case
/// artifact references nor derives a manifest.
pub(crate) async fn check(repository: &Repository) -> Result<CorpusSummary> {
    let schema_root = SCHEMA_ROOT.parse::<RepoPath>()?;
    let case_root = CASE_ROOT.parse::<RepoPath>()?;
    let oracle_root = ORACLE_ROOT.parse::<RepoPath>()?;

    let schema_files = inventory_json(repository, &schema_root)?;
    let case_files = inventory_json(repository, &case_root)?;
    let oracle_files = inventory_json(repository, &oracle_root)?;

    if schema_files.len() != SCHEMA_FILES.len() {
        bail!(
            "conformance schema inventory contains {} JSON files; expected {}",
            schema_files.len(),
            SCHEMA_FILES.len()
        );
    }
    if case_files.len() != EXPECTED_RECORDS || oracle_files.len() != EXPECTED_RECORDS {
        bail!(
            "corpus cardinality differs: {} cases and {} oracles; expected {EXPECTED_RECORDS} each",
            case_files.len(),
            oracle_files.len()
        );
    }

    let stable = StableSelta::new();
    let (case_schema, oracle_schema) =
        admit_schema_inventory(&stable, &schema_root, &schema_files)?;
    let cases = relative_inventory(&case_root, &case_files)?;
    let oracles = relative_inventory(&oracle_root, &oracle_files)?;
    if cases.len() != oracles.len() || !cases.keys().eq(oracles.keys()) {
        bail!("case and oracle inventories do not form an exact relative-path bijection");
    }

    let mut case_ids = BTreeMap::new();
    let mut oracle_ids = BTreeMap::new();
    let mut categories = BTreeMap::new();
    let mut operations = BTreeMap::new();
    let mut oracle_kinds = BTreeMap::new();

    for (relative, case_file) in &cases {
        let oracle_file = oracles
            .get(relative)
            .context("paired oracle disappeared from a closed inventory")?;
        let stem = record_stem(relative.as_str())?;

        let case = parse_ijson(&case_file.bytes, ParseLimits::SCHEMA)
            .with_context(|| format!("cannot parse case record {}", case_file.path))?;
        stable
            .verify_value(&case_schema, &case)
            .await
            .with_context(|| {
                format!(
                    "case record fails stable Selta admission: {}",
                    case_file.path
                )
            })?;
        let case_id = required_string(&case, "id", "case")?;
        if case_id != stem {
            bail!("case ID {case_id} differs from its file stem {stem}");
        }
        if let Some(prior) = case_ids.insert(case_id.to_owned(), relative.clone()) {
            bail!("case ID {case_id} occurs at both {prior} and {relative}");
        }
        let operation = nested_kind(&case, "operation", "case operation")?;

        let oracle = parse_ijson(&oracle_file.bytes, ParseLimits::SCHEMA)
            .with_context(|| format!("cannot parse oracle record {}", oracle_file.path))?;
        stable
            .verify_value(&oracle_schema, &oracle)
            .await
            .with_context(|| {
                format!(
                    "oracle record fails stable Selta admission: {}",
                    oracle_file.path
                )
            })?;
        let oracle_case = required_string(&oracle, "case", "oracle")?;
        if oracle_case != stem || oracle_case != case_id {
            bail!("oracle case {oracle_case} differs from paired case and file stem {stem}");
        }
        if let Some(prior) = oracle_ids.insert(oracle_case.to_owned(), relative.clone()) {
            bail!("oracle case {oracle_case} occurs at both {prior} and {relative}");
        }
        let expected = nested_kind(&oracle, "expected", "oracle expected value")?;
        check_pair_kind(operation, expected, case_id)?;

        increment(&mut categories, category(relative.as_str()));
        increment(&mut operations, operation);
        increment(&mut oracle_kinds, expected);
    }

    Ok(CorpusSummary {
        schemas: schema_files.len(),
        cases: case_files.len(),
        oracles: oracle_files.len(),
        categories,
        operations,
        oracle_kinds,
    })
}

fn inventory_json(repository: &Repository, root: &RepoPath) -> Result<Vec<InventoriedFile>> {
    let directory = repository
        .open_directory(root)
        .with_context(|| format!("cannot open inventory root {root}"))?;
    inventory_recursive(&directory, root, INVENTORY_LIMITS, |path| {
        path.as_str().ends_with(".json")
    })
    .with_context(|| format!("cannot inventory {root}"))
}

fn admit_schema_inventory(
    stable: &StableSelta,
    root: &RepoPath,
    files: &[InventoriedFile],
) -> Result<(AdmittedSchema, AdmittedSchema)> {
    let expected = SCHEMA_FILES.into_iter().collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();
    let mut case_schema = None;
    let mut oracle_schema = None;

    for file in files {
        let relative = strict_relative(root, &file.path)?;
        if relative.as_str().contains('/') {
            bail!("conformance schema is nested below its closed root: {relative}");
        }
        found.insert(relative.to_string());
        let admitted = stable
            .admit_schema(&file.bytes)
            .with_context(|| format!("cannot admit conformance schema {}", file.path))?;
        match relative.as_str() {
            "case.schema.json" => case_schema = Some(admitted),
            "oracle.schema.json" => oracle_schema = Some(admitted),
            _ => {}
        }
    }

    if found.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        bail!("conformance schema inventory differs from the closed 15-file set");
    }
    Ok((
        case_schema.context("case schema missing from admitted inventory")?,
        oracle_schema.context("oracle schema missing from admitted inventory")?,
    ))
}

fn relative_inventory<'a>(
    root: &RepoPath,
    files: &'a [InventoriedFile],
) -> Result<BTreeMap<RepoPath, &'a InventoriedFile>> {
    let mut relative = BTreeMap::new();
    for file in files {
        let path = strict_relative(root, &file.path)?;
        if relative.insert(path.clone(), file).is_some() {
            bail!("inventory path occurs more than once: {path}");
        }
    }
    Ok(relative)
}

fn strict_relative(root: &RepoPath, path: &RepoPath) -> Result<RepoPath> {
    path.strict_relative_to(root)
        .with_context(|| format!("inventory path {path} is not strictly below {root}"))
}

fn record_stem(relative: &str) -> Result<&str> {
    relative
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix(".json"))
        .filter(|stem| !stem.is_empty())
        .with_context(|| format!("record path has no canonical JSON stem: {relative}"))
}

fn required_string<'a>(value: &'a Value, field: &str, label: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("{label} field {field} did not project as a string"))
}

fn nested_kind<'a>(value: &'a Value, field: &str, label: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(|nested| nested.get("kind"))
        .and_then(Value::as_str)
        .with_context(|| format!("{label} did not project a kind"))
}

fn check_pair_kind(operation: &str, expected: &str, case: &str) -> Result<()> {
    let compatible = match operation {
        "admit_environment" => matches!(expected, "admitted" | "rejected"),
        "assess" => matches!(expected, "returned" | "trap"),
        "finite_world" => expected == "finite_result",
        "compare" => expected == "comparison_pass",
        _ => false,
    };
    if !compatible {
        bail!("case {case} operation {operation} is incompatible with oracle kind {expected}");
    }
    Ok(())
}

fn category(relative: &str) -> &str {
    relative.split_once('/').map_or("<root>", |(name, _)| name)
}

fn increment(counts: &mut BTreeMap<String, usize>, value: &str) {
    *counts.entry(value.to_owned()).or_default() += 1;
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::*;

    fn retained_repository_root() -> PathBuf {
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..")).unwrap()
    }

    fn run(repository_root: &Path) -> Result<CorpusSummary> {
        let repository = Repository::open(&fs::canonicalize(repository_root)?)?;
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(check(&repository))
    }

    fn copy_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source = entry.path();
            let destination = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&source, &destination);
            } else {
                fs::copy(source, destination).unwrap();
            }
        }
    }

    fn retained_corpus_copy() -> tempfile::TempDir {
        let temporary = tempfile::tempdir().unwrap();
        let repository = retained_repository_root();
        for relative in [CASE_ROOT, ORACLE_ROOT, SCHEMA_ROOT] {
            copy_tree(&repository.join(relative), &temporary.path().join(relative));
        }
        temporary
    }

    #[test]
    fn retained_corpus_admits_and_matches_the_frozen_acceptance_matrix() {
        let summary = run(&retained_repository_root()).unwrap();

        assert_eq!(
            (summary.schemas, summary.cases, summary.oracles),
            (15, 163, 163)
        );
        assert_eq!(
            summary.categories,
            BTreeMap::from([
                ("admission".to_owned(), 4),
                ("boundary".to_owned(), 32),
                ("finite".to_owned(), 10),
                ("judgment".to_owned(), 35),
                ("laws".to_owned(), 1),
                ("mechanics".to_owned(), 28),
                ("positive-controls".to_owned(), 25),
                ("presence".to_owned(), 28),
            ])
        );
        assert_eq!(
            summary.operations,
            BTreeMap::from([
                ("admit_environment".to_owned(), 4),
                ("assess".to_owned(), 144),
                ("compare".to_owned(), 5),
                ("finite_world".to_owned(), 10),
            ])
        );
        assert_eq!(
            summary.oracle_kinds,
            BTreeMap::from([
                ("admitted".to_owned(), 4),
                ("comparison_pass".to_owned(), 5),
                ("finite_result".to_owned(), 10),
                ("returned".to_owned(), 143),
                ("trap".to_owned(), 1),
            ])
        );
    }

    #[test]
    fn real_corpus_copy_rejects_pair_id_and_kind_mutations() {
        let moved = retained_corpus_copy();
        let oracle = moved
            .path()
            .join("conformance/evidence/candidate-1/oracles/presence/presence-neither.json");
        let nested = oracle.parent().unwrap().join("nested");
        fs::create_dir(&nested).unwrap();
        fs::rename(&oracle, nested.join("presence-neither.json")).unwrap();
        assert!(format!("{:#}", run(moved.path()).unwrap_err()).contains("path bijection"));

        let wrong_id = retained_corpus_copy();
        let case = wrong_id
            .path()
            .join("conformance/evidence/candidate-1/cases/presence/presence-neither.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&case).unwrap()).unwrap();
        value["id"] = Value::String("presence-renamed".to_owned());
        fs::write(&case, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(format!("{:#}", run(wrong_id.path()).unwrap_err()).contains("file stem"));

        let wrong_kind = retained_corpus_copy();
        let oracle = wrong_kind
            .path()
            .join("conformance/evidence/candidate-1/oracles/presence/presence-neither.json");
        fs::write(
            &oracle,
            serde_json::to_vec(&json!({
                "revision": "selta.evidence.conformance-oracle/s2-1",
                "case": "presence-neither",
                "expected": { "kind": "comparison_pass" }
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(format!("{:#}", run(wrong_kind.path()).unwrap_err()).contains("incompatible"));
    }
}
