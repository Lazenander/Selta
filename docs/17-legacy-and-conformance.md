# 17 — Legacy bridge and conformance

> **Status: S2 compatibility boundary.** This document defines how the
> candidate evidence language may observe Selta 0.1 without changing or
> reimplementing it. A serialized legacy capsule, native trace protocol, and
> L4 differential suite are deliberately later work. This document does not
> authorize integration into `selta-core`, protocol 1, or `seltad`.

## One executor, two projections

The existing 0.1 engine remains the sole verifier and sole producer of the
legacy `Report`:

```text
Selta 0.1 inputs -> existing verify engine -> existing Report
                         |
                         +-> factual operational observations, when available
```

The evidence layer may interpret recorded observations later. It MUST NOT
recreate traversal, union selection, combinator folding, sampling, delta
recursion, caching, budget accounting, or report projection in a second engine.

Legacy compatibility and epistemic interpretation are different outputs:

```text
legacy authority: exact existing Report
research output:  evidence AssessmentOutcome under an explicit basis
```

An evidence outcome never replaces or silently redefines the legacy report.

## LegacyExecutionCapsule

A future adapter may produce a `LegacyExecutionCapsule`. This document fixes
what that capsule may claim; it intentionally does not yet define its wire
schema.

Schema provenance is exactly one of:

| Kind | Claim |
|---|---|
| `raw_admitted` | The exact retained source bytes passed duplicate-safe strict admission; their digest and the resulting node snapshot are bound |
| `normalized_admitted` | The exact retained normalized source bytes were admitted and executed; any earlier submitted spelling is unavailable or explicitly non-authoritative |
| `constructed_node` | A node snapshot was supplied programmatically; its serialization may be bound, but it carries no raw-source or strict-admission claim |

This distinction is required because the public 0.1 verifier accepts a `Node`;
it does not require raw admission.

Subject to its observation level, a capsule binds:

- exact engine source identity;
- `selta.schema-language/1`, `selta.meta-validator/1`, and host protocol 1;
- one schema-provenance variant above;
- exact input variant and source or value;
- environment;
- effective `Options` supplied to `verify`, plus requested or clamped values
  only when their boundary was observed;
- a normalized, name-sorted declaration projection containing every
  serializable `ExtensionDecl` field and semantic revision;
- explicit `unknown` markers for an opaque `config_preflight`, host binding,
  or build identity unless an owning boundary supplied their identities;
- settings fingerprints plus sealed identity or an explicit `unknown` marker;
- trace visibility level;
- the parsed `Report` value;
- optional report wire bytes only when captured at a named serialization
  boundary together with its serializer identity; and
- optional boundary or native trace events.

A settings fingerprint is one cache-key component, not a complete replay
identity. If a secret-bearing configuration cannot be preserved safely, the
capsule says that exact replay is unavailable rather than claiming it from the
fingerprint.

Once separately specified and admitted, the capsule will be a typed document
that may appear in an evidence package. It is not a new field on `Node`,
`VerifierSpec`, `Options`, `Report`, or the protocol-1 envelope.

## Trace visibility

Trace visibility and any scoped accounting attestation are orthogonal. A
capsule uses exactly one trace level:

| Level | Honest meaning |
|---|---|
| `report_only` | Available invocation bindings and the parsed final report; optional wire bytes only if actually captured; no claim about discarded internal events |
| `boundary_observed` | Report plus events visible through wrapped host, cache, settings, and monitoring boundaries |
| `native_complete` | Every event required by the pinned engine semantics was emitted by native instrumentation |

Current 0.1 reports discard passing samples, unselected branches, some
combinator details, cache decisions, and nested delta-validation diagnostics.
The current monitor lacks enough path, call, configuration, result, and cache
identity to reconstruct them.

Therefore:

- an external adapter can honestly produce `report_only`;
- selected wrappers can produce `boundary_observed`;
- only a future generic engine `TraceSink` can produce `native_complete`; and
- absent events at a lower level are `not_observed`, never evidence that the
  events did not occur.

Every capsule fact also states its observational origin:

- `native_observation` means the fact was emitted by the engine or captured at
  the exact runtime boundary that owns it; and
- `report_projection` means the fact was copied or mechanically selected from
  the final `Report` and makes no claim about a discarded internal event.

A `report_projection` MUST NOT be relabelled as a `native_observation`. Any
field unavailable at the declared trace level is omitted or explicitly
`unknown`. An adapter MUST NOT rerun verification, resolve `$env` again,
simulate a cache or vote, or reproduce traversal to fill a missing fact.

Adding a `TraceSink` later is an instrumentation change to the one engine, not a
second evaluator. It requires its own compatibility notice and tests.

## Binding the 0.1 profile

The exact 0.1 behavior remains defined by documents 02 through 05 and the
pinned source baseline. A capsule records the following only to the extent
allowed by its trace level:

- intake mode and whether the input was text or an already parsed value;
- `fail_fast`, initial `max_depth`, shared `max_samples`, relative deadline,
  and report-only `schema_name`;
- original configuration when retained with the schema, and resolved
  configuration only when the resolving or host-call boundary exposed it;
- settings identity and requested root/environment projections;
- path, recursive depth, remaining deadline, and cache decision;
- every visible host call, result, or error;
- attempted, valid, malformed, resampled, and projected samples where visible;
- recursive validation of failing delta values; and
- shared usage and sample-budget observations.

Defaults are bound only after the active caller or server resolves them. The
library defaults and a server's clamped values are not assumed equal.

At `report_only`, none of resolved configuration, host calls, cache decisions,
sample attempts, or recursive delta events is inferred from the report. At
`boundary_observed`, wrappers record only calls they actually receive; they do
not manufacture native sampling-round membership or engine ordinals.

## Report equivalence

The legacy adapter preserves the parsed report value returned by the existing
engine; it does not recompute it from the evidence graph.

| Equivalence | Requirement |
|---|---|
| Byte | When wire bytes were actually captured, preserve those bytes and bind the serialization boundary and serializer identity |
| Semantic | Equal schema attribution, extension fingerprints, verdict, deltas, notices, errors, tree, and usage |
| Excluded from semantic equality | Wall-clock start and elapsed timing only |

The core verifier itself returns a `Report`, not bytes. Absence of captured
wire bytes is `not_observed`; an adapter MUST NOT serialize the report later and
present those bytes as the original wire representation. Ordering inside each
report field remains part of the 0.1 contract. A fixture may relax byte
equality only when its manifest states the exact allowed serialization
difference before running the comparison.

## Safe evidence mapping

At `report_only`, the minimal legacy observation is:

```text
the existing engine returned this parsed Report value
```

Only a boundary or native trace may additionally establish:

```text
this exact extension execution returned this protocol-1 envelope
```

It is not:

```text
the underlying semantic proposition is true or false
```

A final K3 verdict is available as a `report_projection`. A host pass, host
delta, or cache hit is a `native_observation` only when its owning boundary was
observed. An extractor or interpreter may give either kind epistemic meaning
only under explicit claims, trust, assumptions, and rules.

The existing `VotePolicy` and K3 behavior can be described by a legacy
projection profile for analysis, but the authoritative legacy result remains
the preserved `Report`. `both` and `neither` in a richer state may both map to
legacy `inconclusive`; their distinction remains available beneath that
projection.

## Required native trace vocabulary

If `native_complete` is ever implemented, every event has a revision, run ID,
event ID, causal parent, deterministic ordinal, logical path, recursive depth,
and typed payload reference. The minimal event families are:

```text
run and intake
node, child, structure, union, and verifier-spec traversal
configuration, settings, and extension binding
cache decision
sample budget and host attempt
delta validation
vote and combinator projection
fail-fast transition
report projection
```

Concurrent samples use causal IDs and attempt ordinals. Wall-clock timestamps
cannot define normative order.

The event vocabulary is intentionally semantic rather than platform-specific:
it contains no process ID, signal, native path, socket, or operating-system
error number.

## Conformance layers

### L0 — schema source

- every candidate schema passes strict Selta admission;
- unknown keys, duplicate keys, wrong discriminants, invalid digests, empty
  required sets, negative or out-of-range limits fail at stable pointers; and
- examples in document 15 validate under the same schema sources.

### L1 — canonical identity

- RFC 8785 canonicalization vectors pass;
- object-key permutations have equal identity;
- normative arrays reject noncanonical order;
- contract or revision changes change identity;
- distinct domain tags never alias;
- digest mismatches and collision simulations at input, generated-state, and
  concluded-outcome insertion fail closed; and
- low-entropy hash disclosure is documented, not presented as secrecy.

### L2 — relational admission

- references close exactly from the package root;
- graph edges are acyclic and within bounds;
- attempt causality, decision information, disposition, and stop closure hold;
- atoms reference observed included attempts;
- assumptions and trust roles resolve; and
- missing or unknown semantics fail before interpretation.

### L3 — algebraic laws

- replay;
- provenance embedding;
- exact duplicate rejection and canonical-set presence invariance;
- conflict and polarity preservation;
- no implicit independence;
- no hidden modality conversion;
- operational/evidence separation;
- authority non-escalation under the caller-pinned assessment basis;
- stable Selta 0.1 boundary non-interference;
- non-empty projection; and
- fuel-bounded termination.

### Deferred L4 — legacy differential

L4 byte-level legacy differential equivalence is not an S2 deliverable. S2
requires only that the stable implementation and protocol remain unchanged and
that the candidate leaves a non-duplicating adapter boundary. L4 begins only
after separate review accepts:

- a closed legacy-capsule schema with provenance and `unknown` variants;
- report-value and optional wire-observation contracts;
- serializer identity and byte-equivalence rules;
- boundary and native event schemas; and
- fixtures that can be produced without reimplementing the engine.

The following is the future acceptance inventory, not a claim that an L4 wire
contract or suite already exists.

Golden capsules cover at least:

```text
clean pass
structural and semantic failure
inconclusive configuration or host result
cache hit and miss
malformed delta and nested delta validation
sampling with resampling and exhausted quorum
shared sample-budget exhaustion
union first pass, all-fail best branch, and inconclusive branch
all_of, any_of, not, skip, and fail-fast
usage overflow
```

Every future case preserves the exact report value and declares its trace
level. Byte comparison applies only to a captured wire observation. No fixture
infers a missing internal event at `report_only` or `boundary_observed`.

### L5 — empirical falsification

The countermodels in document 14 become named fixtures:

```text
CASCADE-FALSE-REJECT
CORRELATED-MAJORITY
CORRELATED-UNANIMITY
INCOMPETENT-INDEPENDENT
RECURSIVE-FIXED-POINT
OPTIONAL-STOP
POST-SELECTION
ALIASED-VOTES
CONFLICT-VS-ABSENCE
BEST-OF-N-PROXY
GENUINE-DISAGREEMENT
OPERATIONAL-COERCION
VALID-LOG-OMISSION
ADAPTIVE-HOLDOUT
```

The positive controls are `CHECKED-WITNESS` and
`CALIBRATED-SEQUENTIAL-EVIDENCE`. A calculus that rejects every amplification
path is as incomplete for the research goal as one that accepts every vote.

## Independent implementation criterion

S2 is not complete merely because one implementation matches its own fixtures.
It therefore permits two isolated conformance implementations before the S3
reference crate. Each implementer initially receives only:

- documents 10 through 23;
- the candidate Selta schemas;
- canonicalization vectors;
- artifact resolver inventories and environment manifests;
- every digest-resolved normative schema and semantic-specification artifact
  named by those manifests;
- input packages and law-case inputs, but no expected outcomes or intermediate
  semantic values; and
- this compatibility boundary, without an inferred legacy capsule wire format.

The implementers work without the temporary corpus generator, one another's
source, generated types, or shared candidate helpers. Each source identity and
prediction ledger is sealed before held-out outcomes are revealed. They must
predict every evidence admission result, stable error location, state,
dependency closure, conclusion set, execution set, and resource counter without
reading another candidate implementation. Exact legacy capsule serialization
and L4 differential equivalence retain their later independent-review gate.

The models live only below `conformance/evidence/candidate-1/`, are not Cargo
workspace members or installable packages, and expose no daemon, protocol, or
application entry point. Stable Selta may provide its already-authoritative
schema admission and value-verification boundary; raw parsing, candidate
identity, relational admission, semantic dispatch, closures, resources, errors,
and output construction remain independently implemented. No production crate
or downstream project may depend on either model.

Each model records its own truthful implementation-artifact identity. Complete
successful outcome bytes may therefore differ only where document 16 permits
execution artifacts to differ. A semantics-free arbiter verifies each package
independently and compares the exact `ConformanceProjection` from document 16.
It never substitutes the manual specification-oracle identity for executable
code.

Before predictions are sealed, each model is packaged as one exact executable
byte artifact. Its implementation identity is `HB` over those bytes under the
document-16 implementation domain. Multi-file source trees may be retained for
review, but neither a directory walk nor an implementation-chosen archive is
an executable identity preimage.

Documentation, schemas, the digest-to-path artifact index, and conformance
vectors are the complete public contract; one implementation cannot establish
an omitted rule for the other. A DSL concept survives S2 only when it is needed
to distinguish a named countermodel, state a law, bind authority or resources,
or execute a mandatory conformance profile. Redundant convenience forms and
implementation-shaped fields are removed rather than explained as aliases.

Any disagreement is a specification defect until resolved in the documents and
fixtures.
