# 21 — S2 conformance execution plan

> **Status: active S2 plan, not normative evidence semantics.** Document 16 is
> the sole numbered law inventory. This document maps those laws to executable
> evidence and fixes the order in which the remaining gate is closed.

## Stage boundary

S2 has three internal steps:

```text
S2a  freeze public grammar and semantics
S2b  complete law, error, mechanics, and falsification corpora
S2c  seal and compare two isolated black-box conformance models
S3   consider the durable Rust reference crate
```

An S2 conformance model is disposable test evidence. It is not a library,
daemon, protocol implementation, application dependency, or prototype for the
S3 crate. No production code may import it. This distinction permits an actual
independence test without making the S2 gate depend on work that the gate
itself forbids.

S2 stable-boundary evidence is likewise narrow: stable Selta source, behavior,
and protocol remain untouched, and the candidate leaves a non-duplicating
adapter seam. Exact legacy capsule serialization and byte-level runtime
differential equivalence remain L4 work.

## Numbered-law coverage

“Positive” demonstrates the required behavior. “Falsifier” distinguishes a
nonconforming implementation that could otherwise pass the positive case.

| # | Law | Current evidence | Required completion |
|---:|---|---|---|
| 1 | Closed admission | Closed positive packages and duplicate-document rejection | Unknown field, revision, contract, and reference cases at their exact first phases |
| 2 | Exact replay | One byte-fixed outcome per positive package | Repeated execution and canonically equivalent raw spelling with a byte-equal complete outcome package from the same model and implementation artifact |
| 3 | Semantic replaceability | Manual specification oracle only | Two sealed complete models; compare the exact `ConformanceProjection` while permitting truthful implementation-artifact differences |
| 4 | Provenance reconstruction | Zero-premise derivation closures and bad-closure mutation | Recursive, atom, acquisition, source, recorder, and attestation closure branches |
| 5 | Duplicate safety | Duplicate document rejection | Duplicate-node rejection and one-support versus two-support state-name/projection invariance without collapsing the changed provenance basis |
| 6 | No inferred independence | No ambient independence field | Two distinct same-stance nodes retain both IDs: state bytes change with the provenance set, while the four-way state name and projected labels do not |
| 7 | Conflict preservation | `neither` and `both` have equal labels but distinct state/provenance | Named case and explicit state-collapse mutation |
| 8 | Polarity explicitness | Four presence states | Missing-stance rejection and absence-as-support mutation |
| 9 | No hidden modality coercion | Rule vectors preserve modality | Submitted wrong-modality semantic mismatch |
| 10 | Operational separation | No acquisition semantics in the presence profile | Separate mechanics profile: unavailable/excluded attempts stay operational and source payloads remain opaque |
| 11 | Assumption visibility | Positive closures and wrong available assumption | Unavailable-assumption error and omitted-closure-assumption mutation |
| 12 | Authority non-escalation | Projection authorization and target pinning | Caller-root, source, recorder, and attestation authority branches |
| 13 | Boundedness | Exact positive counters | Static attempt/node/edge/fan-in/depth limits; call precheck; fuel crossing; joint state/document/byte limits; label limit; outer bytes; and parser depth/value/string limits with exact commits |
| 14 | Canonical identity | Recomputed IDs and duplicate document | Order, mismatch, domain, revision, distinct-node non-collapse, and separate collision injection at input, generated-state, and concluded-outcome insertion |
| 15 | Non-empty projection | Positive labels and empty-label schema failure | Complete |

Presence-only additions come first because they reuse the immutable profile and
add no semantic concept: canonical replay, two-support invariance,
duplicate-node rejection, the four closed-admission boundaries, wrong modality,
unavailable assumption, omitted closure dependency, fuel exhaustion, and
explicit conflict/absence mutations.

Boundedness cases retain first-failure precision. One R1f case may jointly
exceed `max_attempts_total`, `max_graph_nodes`, `max_graph_edges`,
`max_fan_in`, and `max_graph_depth`; one T2 case may jointly exceed
`max_documents`, `max_total_document_bytes`, and `max_state_bytes`. Call
precheck, fuel crossing, J1 labels, outer bytes, and P1 parser depth, value, and
decoded-string ceilings remain separate cases because their phases, commits,
or first-error rules differ.

Cryptographic collision cases use an explicitly declared harness-injected hash
result at each of the three insertion boundaries. Ordinary package bytes are
never presented as a real SHA-256 collision. Each injection must preserve the
two unequal canonical preimages and check the exact `identity`, `interpret`, or
`output` phase and pointer fixed by document 20.

## Mechanics conformance profile

The minimal presence profile MUST NOT grow acquisition machinery merely to
exercise the generic language. A separate `mechanics-candidate-1` profile will
reuse its claim theory, interpreter, and projection and add only total,
deterministic conformance relations:

- one-attempt scheduling;
- deterministic include/exclude selection;
- deterministic stopping;
- scoped-attestation checking;
- exact observation extraction; and
- one explicit operational-fact rule; and
- one one-premise forwarding rule for recursive provenance closure.

Its smallest complete cases are:

1. an `observed + include` attempt feeding an atom;
2. the same atom without its source grant;
3. an `unavailable + exclude` attempt consumed through an `AttemptFact` rule
   for a separate service-health target, while the semantic target stays
   `neither`;
4. an invalid atom over that unavailable attempt;
5. included support plus excluded refutation, with both attempts retained;
6. separate missing-recorder and missing-attestation-semantic authority cases;
7. a derivation over another `EvidenceFact`, proving recursive closure;
8. complete policy, source, recorder, attestation, atom, and acquisition
   dependency closures; and
9. an `observed + exclude` or `malformed + exclude` metamorphic pair whose
   rule inputs differ only through the induced opaque document, attempt, and
   fact identities while the specified rule output remains equal, plus a
   resolver read trap and forbidden payload-dependent output.

This profile exists only below the conformance tree and is not a recommended
application policy.

## Named falsification corpus

The document-14 cases become exact named inputs:

```text
CASCADE-FALSE-REJECT       CORRELATED-MAJORITY
CORRELATED-UNANIMITY       INCOMPETENT-INDEPENDENT
RECURSIVE-FIXED-POINT      OPTIONAL-STOP
POST-SELECTION             ALIASED-VOTES
CONFLICT-VS-ABSENCE        BEST-OF-N-PROXY
GENUINE-DISAGREEMENT       OPERATIONAL-COERCION
VALID-LOG-OMISSION         ADAPTIVE-HOLDOUT
CHECKED-WITNESS            CALIBRATED-SEQUENTIAL-EVIDENCE
```

Finite algebraic cases use one generic declarative finite-world evaluator with
integer weights, deterministic transitions, selectors, and aggregators; they
do not add sixteen bespoke evaluators. Exact rational expectations use reduced
integer pairs when they fit `SafeInt`. Larger values, including
`ADAPTIVE-HOLDOUT`, use a closed factored-BigNat expression whose nonnegative
integers are canonical decimal strings. `VALID-LOG-OMISSION` uses two distinct
worlds with one identical exposed log. Acquisition cases point to
mechanics-profile packages. `CHECKED-WITNESS` requires a separately typed
checker/rule case.
`CALIBRATED-SEQUENTIAL-EVIDENCE` at S2 demonstrates only that all required
population, frame, dependence, calibration, history, statistic, and stopping
assumptions are bound and recoverable in the basis and dependency closure;
advertising an actual calibrated assurance profile remains a deferred scope
decision.

Every serialized case has a Selta schema. A Markdown assertion or an ordinary
unit test is not counted as the named fixture.

## Independent-model protocol

Each implementer receives a hashed kit containing only:

- the frozen normative documents and candidate schemas;
- artifact resolver inventories and environment manifests;
- canonicalization vectors;
- package and law-case inputs; and
- the stable Selta schema-admission/value-verification boundary.

The kit excludes expected outcomes, intermediate semantic values, the temporary
corpus generator, generated candidate types, shared semantic helpers, and the
other model. Each implementation source digest and prediction ledger is sealed
before held-out outcomes are revealed.

A third harness contains no assessment semantics. It invokes each model,
verifies every returned package and identity independently, and compares:

- the exact non-serialized `ConformanceProjection` fixed by document 16.

Each executable records its own implementation-artifact identity. Full outcome
digests may therefore differ exactly where semantic replaceability permits;
the harness never substitutes the manual specification-oracle identity.

Each model is sealed as one executable byte artifact before predictions run;
that file is the implementation-identity preimage. The private harness protocol
is one UTF-8 I-JSON request on stdin and one UTF-8 I-JSON response on stdout,
with no other stdout bytes:

```text
AdmitRequest  { protocol, operation: "admit_environment",
                environment_source_base64,
                resolver: [{ digest, domain, bytes_base64 }], implementation }
AdmitResponse { status: "admitted", environment } | { status: "rejected" }

AssessRequest { protocol, operation: "assess", environment_source_base64,
                resolver, implementation, package_source_base64,
                expected_basis, controls }
AssessResponse { status: "returned", package }
             | { status: "trap", trap: { kind, reference? } }
```

`protocol` is `selta.evidence.conformance/1`; `implementation` is the harness-
computed digest of the sealed executable bytes. Request and response schemas
are conformance artifacts, not product protocol. Diagnostics may use stderr;
exit zero requires exactly one response. The runtime/toolchain pair and their
versions MUST be fixed by an independence review before either implementer kit
is released. No model source work begins while that selection is open.

`controls` is absent for ordinary cases and is a closed conformance-only object
for otherwise unobservable boundaries:

```text
controls {
  identity_collisions: [{ boundary: "input | state | outcome",
                          left_preimage_base64, right_preimage_base64,
                          returned_digest }],
  semantic_outputs: [{ call_ordinal, semantics, output }],
  verification_faults: [{ substage, document_index? }],
  forbidden_reads: [DocumentRef]
}
```

An identity override applies only when both exact canonical preimages and the
named insertion boundary match. A semantic override applies to exactly one
scheduled call. A verification fault applies only at its named defensive
boundary. A forbidden read returns `trap` before exposing the value only when
the bound semantic relation attempts that dereference through its dispatch-time
resolver. Package closure, schema verification, identity, and resource
accounting retain their ordinary assessor access. Every declared control must
match exactly once; an unused, multiply matched, or out-of-scope control fails
the conformance case. These controls never enter the candidate package,
semantics, identities, resources, or product interface.

Any disagreement is first a specification defect. It is resolved in the public
documents and new held-out cases, never by copying one model or mechanically
editing toward the revealed expected bytes.

## Promotion evidence

S2 closes only after the repository contains:

- a machine-readable law-to-case manifest with one positive and one falsifier
  for every numbered law;
- a machine-readable classification of every error as ordinary wire-reachable,
  collision/fault-injection-only, or proved unreachable under candidate
  ceilings, with the matching fixture, injection, or proof;
- complete precedence, pointer, input/state/outcome collision-injection, and
  resource-commit coverage;
- the separate mechanics profile and named falsification corpus;
- two sealed complete conformance prediction ledgers with zero unexplained
  semantic disagreement;
- durable formal, privacy, identity, and compatibility review records; and
- a passing stable workspace suite with no inbound dependency from stable or
  downstream code into the conformance models.

Only that evidence may change document 16 from an open candidate gate to a
completed S2 result.

The named S2 algebraic and representability corpus seeds L5 and later S4 work;
it does not constitute empirical falsification of real assessors, calibration,
or product value.
