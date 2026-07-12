//! Generic exact-layout artifact sets.

use std::collections::BTreeSet;
use std::io::Read;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::digest::{bytes_sha256, h, stream_sha256_exact, Digest, RecordRef};
use crate::json::{parse_ijson, render_record, ParseLimits};
use crate::path::{FileKey, PinnedDirectory, PinnedFile, RepoPath};
use crate::stable::{AdmittedSchema, StableSelta};

pub(crate) const REVISION: &str = "selta.evidence.conformance-artifact-set/s2-1";

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArtifactLimits {
    max_record_bytes: usize,
    max_artifacts: usize,
    max_file_bytes: u64,
    max_total_file_bytes: u64,
}

impl ArtifactLimits {
    pub(crate) const fn new(
        max_record_bytes: usize,
        max_artifacts: usize,
        max_file_bytes: u64,
        max_total_file_bytes: u64,
    ) -> Self {
        Self {
            max_record_bytes,
            max_artifacts,
            max_file_bytes,
            max_total_file_bytes,
        }
    }

    fn parse_limits(self) -> Result<ParseLimits> {
        let max_values = self
            .max_artifacts
            .checked_mul(3)
            .and_then(|value| value.checked_add(3))
            .context("artifact count ceiling cannot be represented as a JSON value ceiling")?;
        Ok(ParseLimits::new(self.max_record_bytes, 4, max_values, 1024))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactSet {
    revision: String,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: RepoPath,
    bytes_sha256: Digest,
}

#[derive(Debug)]
pub(crate) struct GeneratedArtifactSet {
    admitted: AdmittedArtifactSet,
    bytes: Vec<u8>,
}

impl GeneratedArtifactSet {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) const fn reference(&self) -> RecordRef {
        self.admitted.reference()
    }

    pub(crate) fn len(&self) -> usize {
        self.admitted.len()
    }

    pub(crate) fn entries(&self) -> impl ExactSizeIterator<Item = (&RepoPath, Digest)> {
        self.admitted.entries()
    }
}

#[derive(Debug)]
pub(crate) struct AdmittedArtifactSet {
    reference: RecordRef,
    record: ArtifactSet,
}

impl AdmittedArtifactSet {
    pub(crate) const fn reference(&self) -> RecordRef {
        self.reference
    }

    pub(crate) fn len(&self) -> usize {
        self.record.artifacts.len()
    }

    pub(crate) fn entries(&self) -> impl ExactSizeIterator<Item = (&RepoPath, Digest)> {
        self.record
            .artifacts
            .iter()
            .map(|artifact| (&artifact.path, artifact.bytes_sha256))
    }
}

/// Derive one record from an explicit member closure. Input order is not
/// semantic; duplicate authored paths are rejected before the record is made.
pub(crate) async fn derive(
    stable: &StableSelta,
    schema: &AdmittedSchema,
    root: &PinnedDirectory,
    paths: impl IntoIterator<Item = RepoPath>,
    record_path: Option<&RepoPath>,
    limits: ArtifactLimits,
) -> Result<GeneratedArtifactSet> {
    let mut bounded_paths = collect_bounded_paths(paths, limits.max_artifacts)?;
    bounded_paths.sort();
    validate_canonical_paths(bounded_paths.iter(), record_path)?;
    let expected_record_bytes = record_size(&bounded_paths)?;
    if expected_record_bytes > limits.max_record_bytes {
        bail!(
            "artifact-set record is {expected_record_bytes} bytes; limit is {}",
            limits.max_record_bytes
        );
    }

    let mut total_bytes = 0_u64;
    let mut artifacts = Vec::with_capacity(bounded_paths.len());
    for path in bounded_paths {
        artifacts.push(Artifact {
            bytes_sha256: observe_file(root, &path, None, limits, &mut total_bytes)?,
            path,
        });
    }

    let record = ArtifactSet {
        revision: REVISION.to_owned(),
        artifacts,
    };
    let value = serde_json::to_value(&record)?;
    stable.verify_value(schema, &value).await?;
    validate_record(&record, record_path, limits)?;
    let bytes = render_record(&value)?;
    if bytes.len() != expected_record_bytes {
        bail!("artifact-set wire-size invariant differs from its rendered record");
    }
    let reference = RecordRef {
        id: h(REVISION, &value)?,
        bytes_sha256: bytes_sha256(&bytes),
    };

    Ok(GeneratedArtifactSet {
        admitted: AdmittedArtifactSet { reference, record },
        bytes,
    })
}

fn collect_bounded_paths(
    paths: impl IntoIterator<Item = RepoPath>,
    max_artifacts: usize,
) -> Result<Vec<RepoPath>> {
    let mut bounded = Vec::new();
    for path in paths {
        if bounded.len() == max_artifacts {
            bail!("artifact set contains more than its entry ceiling of {max_artifacts}");
        }
        bounded.push(path);
    }
    Ok(bounded)
}

fn record_size(paths: &[RepoPath]) -> Result<usize> {
    // RepoPath excludes every byte that JSON strings escape. These literals
    // are therefore the exact JCS framing for the closed ArtifactSet wire
    // shape, with 64 lowercase hexadecimal bytes per SHA-256 value.
    const RECORD_PREFIX: &[u8] = br#"{"artifacts":["#;
    const ARTIFACT_PREFIX: &[u8] = br#"{"bytes_sha256":""#;
    const ARTIFACT_MIDDLE: &[u8] = br#"","path":""#;
    const ARTIFACT_SUFFIX: &[u8] = br#""}"#;
    const RECORD_SUFFIX_PREFIX: &[u8] = br#"],"revision":""#;
    const RECORD_SUFFIX: &[u8] = br#""}"#;
    const SHA256_HEX_BYTES: usize = 64;

    let mut bytes = RECORD_PREFIX.len();
    for (index, path) in paths.iter().enumerate() {
        if index != 0 {
            bytes = bytes.checked_add(1).context("artifact-set size overflow")?;
        }
        for part in [
            ARTIFACT_PREFIX.len(),
            Digest::PREFIX.len(),
            SHA256_HEX_BYTES,
            ARTIFACT_MIDDLE.len(),
            path.as_str().len(),
            ARTIFACT_SUFFIX.len(),
        ] {
            bytes = bytes
                .checked_add(part)
                .context("artifact-set size overflow")?;
        }
    }
    for part in [
        RECORD_SUFFIX_PREFIX.len(),
        REVISION.len(),
        RECORD_SUFFIX.len(),
        1,
    ] {
        bytes = bytes
            .checked_add(part)
            .context("artifact-set size overflow")?;
    }
    Ok(bytes)
}

/// Admit one retained generated record and verify every member through the
/// already pinned set-root handle before exposing either of its identities.
pub(crate) async fn admit(
    stable: &StableSelta,
    schema: &AdmittedSchema,
    root: &PinnedDirectory,
    record_file: &mut PinnedFile,
    record_path: Option<&RepoPath>,
    limits: ArtifactLimits,
) -> Result<AdmittedArtifactSet> {
    let record_key = record_file.key();
    let source = read_record(record_file, limits.max_record_bytes)?;
    let raw_digest = bytes_sha256(&source);
    let value = parse_ijson(&source, limits.parse_limits()?)?;
    if render_record(&value)? != source {
        bail!("artifact-set record is not retained as exact JCS plus LF");
    }
    stable.verify_value(schema, &value).await?;
    let record: ArtifactSet = serde_json::from_value(value)
        .context("artifact-set schema value does not project to its wire type")?;
    validate_record(&record, record_path, limits)?;

    let mut total_bytes = 0_u64;
    for artifact in &record.artifacts {
        let actual = observe_file(
            root,
            &artifact.path,
            Some(record_key),
            limits,
            &mut total_bytes,
        )?;
        if actual != artifact.bytes_sha256 {
            bail!("artifact byte digest differs at {}", artifact.path);
        }
    }

    let value = serde_json::to_value(&record)?;
    Ok(AdmittedArtifactSet {
        reference: RecordRef {
            id: h(REVISION, &value)?,
            bytes_sha256: raw_digest,
        },
        record,
    })
}

fn validate_record(
    record: &ArtifactSet,
    record_path: Option<&RepoPath>,
    limits: ArtifactLimits,
) -> Result<()> {
    if record.revision != REVISION {
        bail!("wrong artifact-set revision");
    }
    if record.artifacts.len() > limits.max_artifacts {
        bail!(
            "artifact set contains {} entries; limit is {}",
            record.artifacts.len(),
            limits.max_artifacts
        );
    }
    validate_canonical_paths(
        record.artifacts.iter().map(|artifact| &artifact.path),
        record_path,
    )
}

fn validate_canonical_paths<'a>(
    paths: impl IntoIterator<Item = &'a RepoPath>,
    record_path: Option<&RepoPath>,
) -> Result<()> {
    let record_folded = record_path.map(|path| path.to_string().to_ascii_lowercase());
    let mut folded = BTreeSet::new();
    let mut previous = None;
    let mut count = 0_usize;
    for path in paths {
        if previous.is_some_and(|prior| prior >= path) {
            bail!("artifact-set paths are not unique and ASCII-lexically ordered");
        }
        let path_folded = path.to_string().to_ascii_lowercase();
        if record_folded
            .as_ref()
            .is_some_and(|record| record == &path_folded)
        {
            bail!("artifact-set record lists itself at {path}");
        }
        if !folded.insert(path_folded) {
            bail!("artifact set contains a case-insensitive path collision");
        }
        previous = Some(path);
        count += 1;
    }
    if count == 0 {
        bail!("artifact set must not be empty");
    }
    Ok(())
}

fn observe_file(
    root: &PinnedDirectory,
    path: &RepoPath,
    record_key: Option<FileKey>,
    limits: ArtifactLimits,
    total_bytes: &mut u64,
) -> Result<Digest> {
    let mut file = root
        .open_regular_file(path)
        .with_context(|| format!("cannot open artifact-set member {path}"))?;
    if record_key.is_some_and(|key| key == file.key()) {
        bail!("artifact-set member {path} is a hard-link alias of its record");
    }
    let file_bytes = file.len();
    *total_bytes = total_bytes
        .checked_add(file_bytes)
        .context("artifact-set aggregate byte count overflow")?;
    if *total_bytes > limits.max_total_file_bytes {
        bail!(
            "artifact set exceeds its aggregate byte ceiling of {}",
            limits.max_total_file_bytes
        );
    }
    stream_sha256_exact(&mut file, file_bytes, limits.max_file_bytes)
        .with_context(|| format!("cannot hash artifact-set member {path}"))
}

fn read_record(file: &mut PinnedFile, max_bytes: usize) -> Result<Vec<u8>> {
    let expected = file.len();
    let max_u64 = u64::try_from(max_bytes).context("record byte ceiling exceeds u64")?;
    if expected > max_u64 {
        bail!("artifact-set record is {expected} bytes; limit is {max_bytes}");
    }
    let capacity = usize::try_from(expected).context("artifact-set record exceeds usize")?;
    let read_limit = max_u64
        .checked_add(1)
        .context("record byte ceiling cannot be checked")?;
    let mut source = Vec::with_capacity(capacity);
    file.by_ref()
        .take(read_limit)
        .read_to_end(&mut source)
        .context("cannot read retained artifact-set record")?;
    if source.len() > max_bytes {
        bail!("artifact-set record grew beyond its byte ceiling");
    }
    if source.len() as u64 != expected {
        bail!("artifact-set record changed length while it was read");
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::path::Path;

    #[cfg(unix)]
    use serde_json::{json, Value};

    use super::*;
    #[cfg(unix)]
    use crate::path::Repository;

    #[cfg(unix)]
    const LIMITS: ArtifactLimits = ArtifactLimits::new(1_048_576, 32, 1_048_576, 2_097_152);

    #[cfg(unix)]
    fn stable_schema() -> (StableSelta, AdmittedSchema) {
        let stable = StableSelta::new();
        let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("schemas/artifact-set.schema.json");
        let schema = stable
            .admit_schema(&fs::read(schema_path).unwrap())
            .unwrap();
        (stable, schema)
    }

    #[cfg(unix)]
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn canonical_paths_size_and_iterator_bounds_are_platform_neutral() {
        let canonical: [RepoPath; 2] = [
            "alpha.txt".parse().unwrap(),
            "nested/alias.txt".parse().unwrap(),
        ];
        assert_eq!(record_size(&canonical).unwrap(), 301);
        assert!(validate_canonical_paths(std::iter::empty::<&RepoPath>(), None).is_err());
        let duplicate: [RepoPath; 2] = ["a".parse().unwrap(), "a".parse().unwrap()];
        assert!(validate_canonical_paths(duplicate.iter(), None).is_err());
        let descending: [RepoPath; 2] = ["b".parse().unwrap(), "a".parse().unwrap()];
        assert!(validate_canonical_paths(descending.iter(), None).is_err());
        let collision: [RepoPath; 2] = ["A".parse().unwrap(), "a".parse().unwrap()];
        assert!(validate_canonical_paths(collision.iter(), None).is_err());
        let member: RepoPath = "a".parse().unwrap();
        let folded_record: RepoPath = "A".parse().unwrap();
        assert!(validate_canonical_paths([&member], Some(&folded_record)).is_err());
        assert!(
            collect_bounded_paths(std::iter::repeat_with(|| "a".parse().unwrap()), 1,).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn generated_set_admits_real_equal_byte_paths() {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir_all(outer.path().join("root/nested")).unwrap();
        fs::write(outer.path().join("root/alpha.txt"), b"same\n").unwrap();
        fs::write(outer.path().join("root/nested/alias.txt"), b"same\n").unwrap();
        let physical = fs::canonicalize(outer.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let runtime = runtime();
        let generated = runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                [
                    "nested/alias.txt".parse().unwrap(),
                    "alpha.txt".parse().unwrap(),
                ],
                None,
                LIMITS,
            ))
            .unwrap();
        assert_eq!(generated.bytes().last(), Some(&b'\n'));
        assert_eq!(generated.len(), 2);
        let canonical_paths = [
            "alpha.txt".parse().unwrap(),
            "nested/alias.txt".parse().unwrap(),
        ];
        assert_eq!(
            record_size(&canonical_paths).unwrap(),
            generated.bytes().len()
        );
        let exact_record_limit = ArtifactLimits::new(generated.bytes().len(), 2, 5, 10);
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                canonical_paths.clone(),
                None,
                exact_record_limit,
            ))
            .is_ok());
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                canonical_paths,
                None,
                ArtifactLimits::new(generated.bytes().len() - 1, 2, 5, 10),
            ))
            .is_err());

        fs::write(physical.join("artifact-set.json"), generated.bytes()).unwrap();
        let mut record = repository
            .open_regular_file(&"artifact-set.json".parse().unwrap())
            .unwrap();
        let admitted = runtime
            .block_on(admit(&stable, &schema, &root, &mut record, None, LIMITS))
            .unwrap();
        assert_eq!(admitted.reference(), generated.reference());
        let entries = admitted
            .entries()
            .map(|(path, digest)| (path.to_string(), digest))
            .collect::<Vec<_>>();
        assert_eq!(entries[0].0, "alpha.txt");
        assert_eq!(entries[1].0, "nested/alias.txt");
        assert_eq!(entries[0].1, entries[1].1);
    }

    #[cfg(unix)]
    #[test]
    fn construction_rejects_empty_duplicate_self_casefold_and_limits() {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join("root")).unwrap();
        fs::write(outer.path().join("root/a"), b"x").unwrap();
        let physical = fs::canonicalize(outer.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let runtime = runtime();

        assert!(runtime
            .block_on(derive(&stable, &schema, &root, [], None, LIMITS))
            .is_err());
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                std::iter::repeat_with(|| "a".parse().unwrap()),
                None,
                ArtifactLimits::new(1024, 1, 1, 1),
            ))
            .is_err());
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["a".parse().unwrap(), "a".parse().unwrap()],
                None,
                LIMITS,
            ))
            .is_err());
        let path: RepoPath = "a".parse().unwrap();
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                [path.clone()],
                Some(&path),
                LIMITS,
            ))
            .is_err());
        let folded_record: RepoPath = "A".parse().unwrap();
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                [path],
                Some(&folded_record),
                LIMITS,
            ))
            .is_err());
        let collision: [RepoPath; 2] = ["A".parse().unwrap(), "a".parse().unwrap()];
        assert!(validate_canonical_paths(collision.iter(), None).is_err());
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["a".parse().unwrap()],
                None,
                ArtifactLimits::new(1024, 1, 1, 1),
            ))
            .is_ok());
        assert!(runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["a".parse().unwrap()],
                None,
                ArtifactLimits::new(1024, 1, 0, 1),
            ))
            .is_err());
        let error = runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["missing".parse().unwrap()],
                None,
                ArtifactLimits::new(0, 1, 1, 1),
            ))
            .unwrap_err();
        assert!(error.to_string().contains("record is"));
    }

    #[cfg(unix)]
    #[test]
    fn admission_rejects_a_hard_link_to_its_own_record() {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join("root")).unwrap();
        fs::write(outer.path().join("root/member"), b"x").unwrap();
        let physical = fs::canonicalize(outer.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let runtime = runtime();
        let generated = runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["member".parse().unwrap()],
                None,
                LIMITS,
            ))
            .unwrap();
        fs::write(physical.join("artifact-set.json"), generated.bytes()).unwrap();
        fs::remove_file(physical.join("root/member")).unwrap();
        fs::hard_link(
            physical.join("artifact-set.json"),
            physical.join("root/member"),
        )
        .unwrap();
        let mut record = repository
            .open_regular_file(&"artifact-set.json".parse().unwrap())
            .unwrap();
        let error = runtime
            .block_on(admit(&stable, &schema, &root, &mut record, None, LIMITS))
            .unwrap_err();
        assert!(error.to_string().contains("hard-link alias"));
    }

    #[cfg(unix)]
    #[test]
    fn ordinary_member_hard_links_remain_distinct_legal_paths() {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join("root")).unwrap();
        fs::write(outer.path().join("root/a"), b"same").unwrap();
        fs::hard_link(outer.path().join("root/a"), outer.path().join("root/b")).unwrap();
        let repository = Repository::open(&fs::canonicalize(outer.path()).unwrap()).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let generated = runtime()
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["b".parse().unwrap(), "a".parse().unwrap()],
                None,
                LIMITS,
            ))
            .unwrap();
        let digests = generated
            .entries()
            .map(|(_, digest)| digest)
            .collect::<Vec<_>>();
        assert_eq!(digests, [digests[0], digests[0]]);
    }

    #[cfg(unix)]
    #[test]
    fn construction_rejects_missing_nonregular_and_symlink_members() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        let outer = tempfile::tempdir().unwrap();
        fs::create_dir_all(outer.path().join("root/dir")).unwrap();
        fs::write(outer.path().join("root/dir/file"), b"x").unwrap();
        symlink("dir/file", outer.path().join("root/linked-file")).unwrap();
        symlink("dir", outer.path().join("root/linked-dir")).unwrap();
        let _socket = UnixListener::bind(outer.path().join("root/socket")).unwrap();
        let repository = Repository::open(&fs::canonicalize(outer.path()).unwrap()).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let runtime = runtime();
        for path in ["missing", "dir", "socket", "linked-file", "linked-dir/file"] {
            assert!(
                runtime
                    .block_on(derive(
                        &stable,
                        &schema,
                        &root,
                        [path.parse().unwrap()],
                        None,
                        LIMITS,
                    ))
                    .is_err(),
                "accepted {path}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn admission_rejects_format_order_digest_and_file_mutations() {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir(outer.path().join("root")).unwrap();
        fs::write(outer.path().join("root/a"), b"a").unwrap();
        fs::write(outer.path().join("root/b"), b"b").unwrap();
        let physical = fs::canonicalize(outer.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let root = repository.open_directory(&"root".parse().unwrap()).unwrap();
        let (stable, schema) = stable_schema();
        let runtime = runtime();
        let generated = runtime
            .block_on(derive(
                &stable,
                &schema,
                &root,
                ["a".parse().unwrap(), "b".parse().unwrap()],
                None,
                LIMITS,
            ))
            .unwrap();
        let source = generated.bytes();

        let mut variants = Vec::new();
        variants.push(source[..source.len() - 1].to_vec());
        let mut two_newlines = source.to_vec();
        two_newlines.push(b'\n');
        variants.push(two_newlines);
        let value: Value = serde_json::from_slice(source).unwrap();
        variants.push(serde_json::to_vec_pretty(&value).unwrap());
        let mut unordered = value.clone();
        unordered["artifacts"].as_array_mut().unwrap().reverse();
        variants.push(render_record(&unordered).unwrap());
        let mut duplicate = value.clone();
        let first = duplicate["artifacts"][0].clone();
        duplicate["artifacts"][1] = first;
        variants.push(render_record(&duplicate).unwrap());
        let mut wrong = value.clone();
        wrong["artifacts"][0]["bytes_sha256"] =
            json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
        variants.push(render_record(&wrong).unwrap());
        let mut wrong_revision = value.clone();
        wrong_revision["revision"] = json!("selta.evidence.conformance-artifact-set/s2-2");
        variants.push(render_record(&wrong_revision).unwrap());
        let mut unknown = value;
        unknown["unknown"] = json!(true);
        variants.push(render_record(&unknown).unwrap());
        let without_close = std::str::from_utf8(&source[..source.len() - 2]).unwrap();
        variants.push(format!("{without_close},\"revision\":\"{REVISION}\"}}\n").into_bytes());

        for variant in variants {
            fs::write(physical.join("artifact-set.json"), variant).unwrap();
            let mut record = repository
                .open_regular_file(&"artifact-set.json".parse().unwrap())
                .unwrap();
            assert!(runtime
                .block_on(admit(&stable, &schema, &root, &mut record, None, LIMITS,))
                .is_err());
        }

        fs::write(physical.join("artifact-set.json"), source).unwrap();
        let mut record = repository
            .open_regular_file(&"artifact-set.json".parse().unwrap())
            .unwrap();
        assert!(runtime
            .block_on(admit(
                &stable,
                &schema,
                &root,
                &mut record,
                None,
                ArtifactLimits::new(source.len() - 1, 2, 1, 2),
            ))
            .is_err());

        fs::write(physical.join("artifact-set.json"), source).unwrap();
        fs::write(physical.join("root/a"), b"changed").unwrap();
        let mut record = repository
            .open_regular_file(&"artifact-set.json".parse().unwrap())
            .unwrap();
        assert!(runtime
            .block_on(admit(&stable, &schema, &root, &mut record, None, LIMITS,))
            .is_err());
    }
}
