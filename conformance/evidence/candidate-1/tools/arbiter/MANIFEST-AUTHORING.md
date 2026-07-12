# Candidate-1 generated-manifest authoring contract

> **Status: derivation contract; no manifest evidence exists yet.** This file
> freezes the authored choices and derivation rules for the candidate-1 S2
> manifest before their implementation. It is arbiter-side procedure, not
> evidence semantics, a public model input, or a claim that
> `manifest-input.json` or `manifest.json` has been retained.

Documents 15, 16, and 20 define the language, semantics, identities, and
errors. Document 21 defines the conformance obligation. `SEALING.md` defines
the record lifecycle. A disagreement is resolved in those sources or this
authoring contract before code is changed; derivation never guesses.

## Fixed files and lifecycle

The sole authored input and sole generated output are, respectively:

```text
conformance/evidence/candidate-1/manifest-input.json
conformance/evidence/candidate-1/manifest.json
```

The arbiter always inventories the exact authored input bytes and admits its
value under
`conformance/evidence/candidate-1/schemas/manifest-input.schema.json`.
The input cannot omit or rename itself through a field. The generated manifest
never inventories itself, directly, through case folding, or through another
path to the same file. No other manifest-input path is accepted.

The input is admitted before discovery. It contains choices only; every hash,
admission, context resolver row, case/oracle pair, expected path, error-case
set, and ordering relation is derived. The generated output is JCS plus one LF
as fixed by `SEALING.md`. Failure is terminal and emits no output.

## Closed discovery

The authored discovery values are exactly:

| Field | Repository-relative path |
|---|---|
| `case_root` | `conformance/evidence/candidate-1/cases` |
| `oracle_root` | `conformance/evidence/candidate-1/oracles` |
| `conformance_schema_root` | `conformance/evidence/candidate-1/schemas` |

These three roots are pairwise disjoint. They are recursively closed: every
regular descendant whose exact repository-relative path ends in `.json` is
included. The current closed inventory is 163 case records, 163 oracle records,
and 15 conformance schemas. These cardinalities are assertions, not discovery
limits; a change aborts until this contract and its evidence are reviewed.

Case and oracle relative paths must form a bijection. Each case `id` and oracle
`case` must equal one another and the file stem. Duplicate IDs, missing peers,
extra peers, or a second path for one ID fail derivation. Cases and oracles are
admitted before their references are followed.

Traversal uses one retained repository handle and component-relative,
no-follow descent. Each directory is enumerated once in raw ASCII lexical
order for an operation. Every encountered entry, including ignored non-JSON
entries, must have an exact canonical ASCII name and be a regular file or
directory; symlinks, special files, case-fold aliases, unreadable entries, and
replacement between enumeration and opening fail. An enumerated entry is
opened from its pinned parent entry, not by a fresh repository path lookup.
Every file is read from its retained handle and verified unchanged after the
complete read. Operational ceilings abort; they do not change closure.

The exact artifact-root antichain is:

```text
conformance/evidence/candidate-1
docs
profiles/evidence
schemas/evidence/candidate-1
```

No listed root is `.`, nested in another listed root, or a file. An authored
exact file is self-bounded. An exact source root has only the recursive meaning
assigned below. No unrelated descendant enters the manifest merely because it
is below an artifact root.

## Contexts and raw inputs

The four context rows, in canonical ID order, are:

| ID | Environment source |
|---|---|
| `boundary-candidate-1` | `profiles/evidence/boundary-candidate-1/environment.json` |
| `mechanics-candidate-1` | `profiles/evidence/mechanics-candidate-1/environment.json` |
| `positive-controls-candidate-1` | `profiles/evidence/positive-controls-candidate-1/environment.json` |
| `presence-candidate-1` | `profiles/evidence/presence-candidate-1/environment.json` |

The exact `raw_only` set is the following 15 paths beneath
`profiles/evidence/boundary-candidate-1/fixtures/raw/`:

```text
boundary-decoded-string-exact.input.raw
boundary-decoded-string-exceeded.input.raw
boundary-depth-exact.input.raw
boundary-depth-exceeded.input.raw
boundary-raw-bytes-exact.input.raw
boundary-raw-invalid-utf8.input.raw
boundary-raw-lone-surrogate.input.raw
boundary-raw-noncharacter.input.raw
boundary-raw-number-out-of-range.input.raw
boundary-raw-syntax.input.raw
boundary-raw-trailing-data.input.raw
boundary-values-exact.input.raw
boundary-values-exceeded.input.raw
precedence-parse-earlier-duplicate-before-later-syntax.input.raw
precedence-parse-outer-before-duplicate.input.raw
```

Prefixing those names with any other directory is not equivalent.

## Resolver closure and identity

The resolver accepts exactly five domains:

| Domain | Identity preimage |
|---|---|
| `selta.evidence.schema-source/candidate-1` | `H` over document 16's canonical admitted-`Node` projection |
| `selta.evidence.language-specification/candidate-1` | `HB` over exact `docs/15-evidence-language.md` bytes |
| `selta.evidence.core-semantics/candidate-1` | `HB` over exact `docs/16-evidence-semantics.md` bytes |
| `selta.evidence.error-catalog/candidate-1` | `HB` over exact `docs/20-evidence-error-catalog.md` bytes |
| `selta.evidence.semantic-specification/candidate-1` | `HB` over exact document 19, 22, or 23 bytes, as referenced by an environment |

Here `H` and `HB`, including their domain separators, are exactly those in
document 16. A schema source is first admitted by stable Selta under the frozen
strict source profile; identity is computed from the admitted projection, not
raw JSON. The four Markdown domains bind exact bytes. A manifest resolver row's
`digest` is this domain-specific identity and its `source` is the ordinary
SHA-256 digest of the exact retained source bytes.

For each environment, closure starts from its language, core-semantics, and
error-catalog sources, every evaluation-extension configuration schema, every
contract schema, and every semantic specification. All internal references
must resolve. The present closures contain 30, 37, 46, and 29 resolver rows for
boundary, mechanics, positive-controls, and presence, respectively. Their
union contains exactly 49 distinct `(domain, digest)` identities: six
specification documents and 43 admitted schema projections.

The authored global source pool contains exactly one path for each of those 49
identities and no unused row. Each context uses the required subset; sharing a
row across contexts is allowed. Two paths with one identity do not create two
resolver rows. The only candidate-1 source ambiguity is the equal admitted
projection and equal raw bytes of the presence `unit` and `non_empty` schemas;
the selected path is exactly:

```text
profiles/evidence/presence-candidate-1/builtin-config-schemas/non_empty.schema.json
```

An implementation artifact, environment identity, or Markdown
`ARTIFACTS.md` inventory row is not a resolver domain. The profile
`ARTIFACTS.md` files are review aids; the arbiter recomputes closure from the
admitted environments and exact authored source paths.

## Admission roles

One artifact may acquire several obligations. Obligations form a canonical
set; no role is inferred from a filename alone.

| Artifact role | Required admission |
|---|---|
| Authored manifest input | `selta_value` under the manifest-input schema |
| Discovered conformance schema | `selta_schema` |
| Resolver schema source | `selta_schema` |
| Context environment source | `selta_value` under `schemas/evidence/candidate-1/environment.schema.json` |
| Case record | `selta_value` under the conformance case schema |
| Oracle record | `selta_value` under the conformance oracle schema |
| Finite program | `selta_value` under the finite-world schema |
| Finite expected result | `selta_value` under the finite-result schema |
| Returned expected package | `selta_value` under `schemas/evidence/candidate-1/package.schema.json` |
| Retained runtime artifact-set record | `selta_value` under the artifact-set schema |
| RFC 8785 vector record | `selta_value` under the JCS-vectors schema |
| Assessment input package or raw parser input | raw bytes only |
| Markdown specification, review, or guidance | raw bytes only |
| Retained runtime member | raw bytes only |

Every referenced raw digest resolves to exactly one manifest artifact. An
artifact entry is unique by ordinary raw digest. Equal bytes reached through
several roles merge their obligations; unequal bytes never merge merely
because their Selta projections or purposes agree.

## Oracle-free public closure

`public_inputs.include` is exactly the following four values in this declared
order:

```text
context_environments
context_resolvers
model_case_records
model_case_sources
```

Their meanings are closed:

- `context_environments` includes all four exact environment sources;
- `context_resolvers` includes the union of their resolver-source artifacts;
- `model_case_records` includes every case record whose operation is
  `admit_environment` or `assess`; and
- `model_case_sources` includes every package or raw source referenced by an
  `assess` case.

Finite-world and comparison case records, finite expected results, all
oracles, expected packages, the generated manifest, reviews, arbiter sources,
and manifest authoring choices are excluded.

`public_inputs.files` is exactly this 22-path set:

```text
conformance/evidence/candidate-1/README.md
conformance/evidence/candidate-1/SEALING.md
conformance/evidence/candidate-1/schemas/case.schema.json
conformance/evidence/candidate-1/schemas/jcs-vectors.schema.json
conformance/evidence/candidate-1/schemas/request.schema.json
conformance/evidence/candidate-1/schemas/response.schema.json
conformance/evidence/candidate-1/vectors/README.md
conformance/evidence/candidate-1/vectors/rfc8785.json
docs/10-evidence-and-decision.md
docs/11-compatibility-and-versioning.md
docs/12-evidence-evaluation-plan.md
docs/13-evidence-research-ledger.md
docs/14-terms-and-counterexamples.md
docs/15-evidence-language.md
docs/16-evidence-semantics.md
docs/17-legacy-and-conformance.md
docs/18-platform-boundaries.md
docs/19-presence-reference-profile.md
docs/20-evidence-error-catalog.md
docs/21-s2-conformance-plan.md
docs/22-mechanics-conformance-profile.md
docs/23-positive-controls-profile.md
```

Both the vector guidance and its Selta schema are intentional members.

## Isolated model additions

Model A receives exactly these additional files:

```text
Cargo.lock
Cargo.toml
crates/selta-core/Cargo.toml
crates/selta-protocol/Cargo.toml
```

Its exact recursive source roots are:

```text
crates/selta-core/src
crates/selta-protocol/src
```

Every regular descendant is included regardless of suffix. No extension
filter, build-script expansion, `include!` expansion, generated source search,
or adjacent-crate inference is permitted. The current roots contain no
`include!`, `include_str!`, `include_bytes!`, or `build.rs` dependency; finding
one aborts until its boundary is authored explicitly. The two roots are
self-bounded and remain independent of operating-system path logic after
pinned discovery.

Model B's authored values are fixed as:

```text
runtime_profile = selta.evidence.conformance-runtime/cpython-3.9-stdlib-1
runtime_root = conformance/evidence/candidate-1/runtime/model-b/cpython-3.9-stdlib-1
runtime_artifact_set_path = conformance/evidence/candidate-1/runtime/model-b/cpython-3.9-stdlib-1.artifact-set.json
```

The root and record do not presently exist. No source-prefix observation or
diagnostic hash substitutes for retained runtime evidence. The exact runtime
identity remains unknown until the `SEALING.md` selection is copied, its
closed member rule is checked, and its artifact-set record is retained and
admitted. This is the current hard blocker for a complete manifest.

## Named corpus mapping

Witness sets are canonical ASCII-lexical case-ID sets.

| Name | Exact witnesses |
|---|---|
| `CASCADE-FALSE-REJECT` | `cascade-false-reject` |
| `CORRELATED-MAJORITY` | `correlated-majority` |
| `CORRELATED-UNANIMITY` | `correlated-unanimity` |
| `INCOMPETENT-INDEPENDENT` | `incompetent-independent` |
| `RECURSIVE-FIXED-POINT` | `recursive-fixed-point` |
| `OPTIONAL-STOP` | `optional-stop` |
| `POST-SELECTION` | `mechanics-two-attempt-predecessor` |
| `ALIASED-VOTES` | `presence-duplicate-node`, `presence-support-multiplicity` |
| `CONFLICT-VS-ABSENCE` | `presence-conflict-vs-absence` |
| `BEST-OF-N-PROXY` | `best-of-n-proxy` |
| `GENUINE-DISAGREEMENT` | `genuine-disagreement` |
| `OPERATIONAL-COERCION` | `mechanics-invalid-atom-unavailable`, `mechanics-unavailable-operational` |
| `VALID-LOG-OMISSION` | `valid-log-omission` |
| `ADAPTIVE-HOLDOUT` | `adaptive-holdout` |
| `CHECKED-WITNESS` | `positive-controls-checked-witness-matching` |
| `CALIBRATED-SEQUENTIAL-EVIDENCE` | `positive-controls-sequential-anytime-valid`, `positive-controls-sequential-fixed-horizon` |

The two-witness rows are irreducible: each binds a distinct required
observation. Named rows retain document 21's declared name order.

## Numbered-law mapping

The following mapping is exact. A repeated case in positive and falsifier sets
is intentional under `SEALING.md`: its retained projection demonstrates the
law and kills a specific nonconforming mutation.

| Law | Positive case IDs | Falsifier case IDs |
|---:|---|---|
| 1 | `presence-neither` | `identity-wrong-revision`; `presence-missing-reference`; `presence-unknown-contract`; `presence-unknown-wrapper-field` |
| 2 | `presence-replay-equality` | `presence-replay-equality` |
| 3 | `semantic-replaceability-cross-model` | `semantic-replaceability-cross-model` |
| 4 | `mechanics-forwarding-recursive`; `positive-controls-sequential-fixed-horizon` | `mechanics-forwarding-recursive`; `positive-controls-sequential-fixed-horizon` |
| 5 | `presence-support-multiplicity` | `presence-duplicate-document`; `presence-duplicate-node`; `presence-support-multiplicity` |
| 6 | `presence-two-support` | `presence-support-multiplicity` |
| 7 | `presence-both` | `presence-conflict-vs-absence` |
| 8 | `presence-both`; `presence-neither`; `presence-refute-only`; `presence-support-only` | `presence-absence-vs-support`; `presence-missing-stance` |
| 9 | `mechanics-forwarding-recursive` | `presence-wrong-modality` |
| 10 | `mechanics-observed-exclude-opaque-a`; `mechanics-observed-exclude-opaque-b`; `mechanics-unavailable-operational` | `mechanics-invalid-atom-unavailable`; `mechanics-opaque-payload-mutant-a`; `mechanics-opaque-payload-mutant-b`; `mechanics-operational-forbidden-read` |
| 11 | `positive-controls-checked-witness-matching`; `positive-controls-sequential-fixed-horizon` | `positive-controls-checked-witness-unavailable-theory-assumption`; `positive-controls-sequential-omitted-assumption`; `positive-controls-sequential-unavailable-assumption` |
| 12 | `mechanics-observed-include-atom`; `mechanics-scoped-accepted`; `positive-controls-sequential-fixed-horizon` | `mechanics-missing-attestation-grant`; `mechanics-missing-recorder`; `mechanics-missing-source-grant`; `positive-controls-checked-witness-missing-checker-grant`; `positive-controls-sequential-missing-recovery-grant`; `positive-controls-sequential-substantive-target-binding`; `positive-controls-sequential-substantive-target-output`; `presence-unauthorized-projection`; `presence-wrong-target`; `relation-basis-mismatch` |
| 13 | `boundary-decoded-string-exact`; `boundary-depth-exact`; `boundary-raw-bytes-exact`; `boundary-values-exact`; `presence-fuel-exact`; `resource-interpret-fuel-exact-project-crossing`; `resource-interpret-state-limits-exact`; `resource-project-labels-exact`; `resource-relation-fuel-exact-interpret-crossing`; `resource-relation-static-exact`; `resource-state-equal-digest-keeps-charge` | `boundary-decoded-string-exceeded`; `boundary-depth-exceeded`; `boundary-values-exceeded`; `mechanics-arithmetic-overflow-interpret`; `mechanics-arithmetic-overflow-project`; `mechanics-arithmetic-overflow-relation`; `output-success-too-large`; `precedence-parse-outer-before-duplicate`; `presence-fuel-crossing`; `resource-interpret-call-limit`; `resource-interpret-fuel-exact-project-crossing`; `resource-interpret-state-limits-exceeded`; `resource-project-call-limit`; `resource-project-labels-exceeded`; `resource-relation-call-limit`; `resource-relation-fuel-crossing`; `resource-relation-fuel-exact-interpret-crossing`; `resource-relation-static-exceeded` |
| 14 | `presence-support-multiplicity`; `resource-state-equal-digest-keeps-charge` | `boundary-unsafe-int`; `control-input-identity-collision`; `control-outcome-identity-collision`; `control-state-identity-collision`; `identity-wrong-domain`; `identity-wrong-revision`; `mechanics-noncanonical-predecessors`; `precedence-identity-mismatch-before-duplicate`; `precedence-identity-noncanonical-before-mismatch`; `presence-support-multiplicity` |
| 15 | `resource-project-labels-exact` | `presence-empty-label-projection` |

Law 4's two projections jointly cover recursive atom/acquisition/source closure
and recorder/attestation/assumption closure. Law 13 deliberately uses the two
exact-then-crossing cases on both sides: they prove equality acceptance in one
phase and the first crossing in the next. Law 14 treats
`presence-support-multiplicity` as both the positive distinct-provenance
observation and the falsifier for illicit alias collapse.

## Error-table closure

The sole error classification source is:

```text
conformance/evidence/candidate-1/reviews/error-reachability.md
```

Its currently frozen raw SHA-256 is
`bbcb04065ae29d370590453d60ab8a036a7458c56269c708ff1410c657534df9`.
The numbered 48-row matrix is closed: 35 rows are `wire`, 13 are `control`, and
zero are `unreachable`. The exact control rows are 08, 11, 16, 23, 32, 34, 36,
37, 39, 41, 43, 45, and 46. Each input classification must match the matrix's
phase, code, reachability, and control kind. The arbiter derives each generated
row's non-empty canonical case-ID set from the same matrix and requires every
listed case to exist. Extra, missing, duplicate, or unreferenced rows fail.

The review is retained as raw Markdown and is never included in the public
model kit. A changed review digest requires this contract to be reviewed; the
arbiter must not parse a changed table under an old expectation.

## Canonical output and emission gate

Sets use raw ASCII lexical order unless this contract fixes another order.
Artifacts are unique by raw digest; contexts and cases order by ID; each
resolver orders by `(digest, domain, source)`; laws order by number; named rows
retain document 21 order; errors retain the closed 48-row schedule. All
references must be used and all discovered or derived artifacts must have the
roles above.

Neither manifest file is retained at present. In particular, the arbiter MUST
NOT emit `manifest.json`, a partial substitute, or a purported fixed manifest
identity until the Model B retained-runtime root and admitted artifact-set
record exist and every rule in this document succeeds in one operation. The
absence of those files is an incomplete construction state, not conformance
evidence and not permission to weaken the derivation.
