# Candidate-1 conformance arbiter

> **Status: isolated S2 evidence tooling.** This program is not Selta product
> code, a candidate assessor, a reusable evidence library, or an S3 reference
> implementation. The repository workspace does not contain it, and no stable
> crate may depend on it.

The arbiter makes the candidate-1 conformance run reproducible without
implementing assessment semantics a third time. Its allowed semantic work is
limited to stable Selta admission, post-hoc verification of returned values
and identities, the generic finite-world DSL, observation selection, and
relations between retained records.

## Trust boundary

The executable has four one-way lanes:

```text
authored choices -> derived manifest -> oracle-free kits -> freeze
sealed model     -> captured runs    -> predictions
withheld oracles -> comparison       -> completion
retained files   -> typed records    -> seals
```

The manifest lane derives hashes, admissions, resolver closure, case/oracle
pairs, and law/named/error coverage. The kit lane starts from an explicit
public closure and applies path-and-byte denials before and after traversal.
The prediction lane can invoke only environment admission and assessment and
cannot open an oracle. The reveal lane cannot begin until both complete
prediction seals admit. Cross-model comparison additionally requires unequal
executable bytes and unequal implementation identities.

The arbiter may check a returned package, recompute its identities, and derive
the exact `ConformanceProjection`. It has no operation that accepts a package
input and constructs candidate states, closures, conclusions, or outcomes.

[`MANIFEST-AUTHORING.md`](MANIFEST-AUTHORING.md) freezes the exact authored
boundary and derivation rules for the next construction step. It is not a
manifest, model input, or substitute for the absent retained runtime evidence.

## Modules

- `main` and `cli` contain command wiring only.
- `json` owns bounded duplicate-safe I-JSON and RFC 8785 JCS.
- `digest` owns raw SHA-256 and domain-separated identities.
- `path` owns repository-relative paths and a root handle retained for the
  whole operation. Its Unix adapter descends with component-relative,
  no-follow opens, requires exact stored component spelling, and rejects
  case-fold aliases. Enumerated entries bind raw names and physical identity to
  the same parent handle before opening; complete entry metadata is compared
  without another directory scan, and file stamps are explicitly finalized
  after complete reads. Other platforms fail closed until their adapter
  provides equivalent semantics; the remaining arbiter logic contains no
  operating-system path calls.
- `records` contains only the conformance wire shapes.
- `stable` is the sole module allowed to import `selta-core`.
- `inventory` owns policy-neutral bounded recursive file closure.
- `corpus` admits and pairs the closed case, oracle, and schema inventories.
- `manifest`, `artifact`, and `kit` derive the generated manifest and
  oracle-free kits.
- `process` and `prediction` own isolated launch and capture.
- `candidate_check` performs post-hoc package checks only.
- `finite` evaluates the closed, generic finite-world DSL.
- `compare`, `seal`, and `evaluate` validate the record DAG and lifecycle.

Candidate identity routines remain private to the executable. Neither model
kit contains the arbiter source, and the two models share no semantic helper.

## Command lifecycle

```text
arbiter check-corpus-inventory
arbiter manifest
arbiter artifact-set
arbiter kit
arbiter seal freeze
arbiter predict
arbiter seal prediction
arbiter evaluate-blind
arbiter evaluate-reveal
arbiter seal completion
```

Commands are added only when their complete invariant can be enforced. A
watchdog, output ceiling, malformed record, or failed relational check aborts
without emitting a ledger or seal; operational failure never becomes a
candidate conformance outcome.

The implemented corpus-inventory command is:

```text
arbiter check-corpus-inventory \
  --repository <physical-repository>
```

It recursively inventories exact `.json` suffixes beneath the three authored
roots, admits all fifteen conformance schemas and every case and oracle through
stable Selta, and requires the frozen `163/163/15` cardinalities, relative-path
bijection, filename IDs, and operation/oracle-kind compatibility. Its category
and kind distributions are derived retained-fixture observations, not a second
hidden authored case list. The command follows no case artifact reference,
writes nothing, and cannot emit a partial manifest.

The implemented generic-set admission command is:

```text
arbiter artifact-set admit \
  --repository <physical-repository> \
  --root <repository-relative-set-root-or-dot> \
  --record <repository-relative-record>
```

It emits one JCS-plus-LF `{ id, bytes_sha256 }` result only after stable Selta
verification, relational canonicality, pinned-handle file checks, streaming
digests, and logical and hard-link self-exclusion all succeed. Its record,
artifact-count, visited-directory-entry, per-file, and aggregate-byte flags are
operational ceilings, not artifact-set semantics. Construction remains
internal until an enclosing manifest, kit, or runtime operation supplies the
closed member list; there is no second authored member-list DSL.

`check-foundation` derives and admits the retained
`fixtures/artifact-set/minimal` tree. That fixture includes a nested member and
two distinct paths with equal bytes, and fixes both typed and exact-byte
identities.

## Construction order

1. Establish the isolated crate, I-JSON/JCS, identities, paths, the stable
   Selta adapter, and conformance record admission.
2. Complete artifact-set and generated-manifest derivation.
3. Build the common kit closure and both isolated model kits.
4. Add process capture, prediction ledgers, and runtime evidence.
5. Add blind validation, oracle reveal, comparisons, and completion sealing.

Each step is accepted by retained real-file tests. Unit tests may diagnose a
component, but they do not by themselves count as conformance evidence.
