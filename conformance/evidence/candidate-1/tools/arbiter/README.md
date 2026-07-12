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

## Modules

- `main` and `cli` contain command wiring only.
- `json` owns bounded duplicate-safe I-JSON and RFC 8785 JCS.
- `digest` owns raw SHA-256 and domain-separated identities.
- `path` owns repository-relative paths and a root handle retained for the
  whole operation. Its Unix adapter descends with component-relative,
  no-follow opens and consumes bytes from the validated file handle. Other
  platforms fail closed until their adapter provides equivalent semantics;
  the remaining arbiter logic contains no operating-system path calls.
- `records` contains only the conformance wire shapes.
- `stable` is the sole module allowed to import `selta-core`.
- `manifest`, `artifact`, and `kit` derive inventories and oracle-free kits.
- `process` and `prediction` own isolated launch and capture.
- `candidate_check` performs post-hoc package checks only.
- `finite` evaluates the closed, generic finite-world DSL.
- `compare`, `seal`, and `evaluate` validate the record DAG and lifecycle.

Candidate identity routines remain private to the executable. Neither model
kit contains the arbiter source, and the two models share no semantic helper.

## Command lifecycle

```text
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

## Construction order

1. Establish the isolated crate, I-JSON/JCS, identities, paths, the stable
   Selta adapter, and conformance record admission.
2. Complete artifact-set and generated-manifest derivation.
3. Build the common kit closure and both isolated model kits.
4. Add process capture, prediction ledgers, and runtime evidence.
5. Add blind validation, oracle reveal, comparisons, and completion sealing.

Each step is accepted by retained real-file tests. Unit tests may diagnose a
component, but they do not by themselves count as conformance evidence.
