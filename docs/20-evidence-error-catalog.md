# 20 — Evidence error catalog

> **Status: S2 normative candidate.** These are the complete candidate-1 error
> codes and fixed messages. Errors are operational results, not evidence or
> conclusion labels.

This catalog governs `assess` only after the caller supplies an admitted
semantic environment. Environment-constructor rejection is an out-of-band
trust-bootstrap failure and has no candidate error record or outcome package.

## Record and ordering

Every record is exactly:

```text
{ phase, code, path, message }
```

`path` is an RFC 6901 pointer into one conceptual assessment workspace:

```text
{ input: <parsed package>, generated: { state, closures, conclusions, resources } }
```

The `input` member becomes available after parsing; generated members appear as
their phases commit them. Parse failures and the truncation marker use the
empty pointer. Every other anchor below includes `/input` or `/generated`, so a
code never silently changes pointer namespace with its phase. Phase order is:

```text
parse < schema < identity < relation < interpret < project < output
```

Only the first failing substage in the phase schedule below emits records.
Duplicate `(phase, code, path)` records are removed, then records are sorted by
`(path UTF-8 bytes, code UTF-8 bytes)`. At most 256 records are returned. If
more exist, return the first 255 sorted records followed by this terminal
marker, which is the only exception to ordinary tuple order:

```json
{
  "phase": "output",
  "code": "error.truncated",
  "path": "",
  "message": "additional errors were omitted"
}
```

The marker does not state an unbounded total count. Before deduplication, every
anchor is normalized to at most 2,048 UTF-8 bytes: while an encoded RFC 6901
pointer is longer, remove its final complete pointer token. A token is never
split. Because all post-parse paths begin with `/input` or `/generated`, this
always leaves a valid non-empty ancestor. This bounds error output even when an
application-defined object uses a very long member name; normalized records may
then deduplicate at their common ancestor.

## Phase and substage schedule

An implementation completes substages in this exact order and stops at the
first substage that emits one or more errors:

1. **Parse P0:** reject a raw byte slice over the outer byte ceiling before
   decoding. **P1:** otherwise parse once with the common depth, value, and
   decoded-string counters from document 16. Return the first event in source
   byte order. At one byte offset, precedence is outer limit, duplicate name,
   then UTF-8 or syntax.
2. **Schema S0:** verify the package wrapper. **S1:** compare `environment` with
   the admitted environment. **S2:** for each document, resolve its contract
   and run stable Selta verification. S2 emits at most one error per document;
   unknown contract suppresses value verification for that document.
3. **Identity I0a:** require every collection with a fixed order in document 15
   to use that order. Canonical-set keys are nondecreasing and equality is
   allowed at this substage; attempts use `(ordinal, completed ID)`. A semantic
   sequence such as ordered rule premises has no additional sort predicate.
   **I0b:** check every declared
   identity and collision. **I0c:** reject duplicate members in the now-ordered,
   identity-valid sets. **I0d:** resolve every semantics ID. **I1:** if clean,
   check every tagged reference for
   existence and target contract. **I2:** if clean, check document-reference
   cycles. **I3:** if clean, emit one error for each unreachable document.
4. **Relation R0:** compare the root with `ExpectedBasisRef`; mismatch stops
   without trusting or executing package-selected semantics. **R1a:** check
   descriptor kind at every binding use. **R1b:** check binding parameter
   contract and specification predicates and, for a claim theory, its
   assessor-built canonicalization input contract and call-domain predicates.
   **R1c:** check exact grants and assumption availability. **R1d:** check local
   decision, attempt, accounting, atom, and derivation fields and submitted
   call-input predicates using only each record and schema/identity-valid
   referenced values.
   **R1e:** check acquisition-wide trace relations and graph inventories.
   **R1f:** check premise acyclicity and static basis resource limits. **R2:**
   if all are clean, execute submitted relations in the schedule from document
   16 and compare each result.
5. **Interpret T0:** dispatch the interpreter and validate its output contract.
   **T1:** wrap the contract-valid state, commit its generated-state counters,
   and compare it with every input document sharing its digest; unequal bytes
   emit `identity.collision`. Equal bytes reuse the input map entry without
   reversing the charge. **T2:** compare all three state-related basis limits.
   **Project J0:** dispatch projection, validate its output, and commit the
   conclusion-label counter. **J1:** compare its basis limit. **Output O0:**
   construct and measure the ordinary outcome, applying the fixed oversized
   replacement below when required. **O1:** before inserting a concluded
   outcome, compare it with every retained document sharing its digest. Equal
   bytes reuse the retained entry; unequal bytes replace the conclusion with an
   `identity.collision` error outcome. Measure that replacement once against
   the same outer cap; an oversized replacement becomes the fixed
   `output.too_large` form. Error
   outcomes retain no input or state documents, so O1 has no peer map entry to
   compare.

Within S2, each I0 substage, I1, I3, and each R1 substage, all records at that substage are
collected from inputs whose prerequisites from earlier substages are valid. A
failure never causes a containing-record error or any later-substage error: for
example, an unavailable assumption emits `relation.invalid_assumption` only,
and a wrong-kind binding emits `relation.kind_mismatch` only. R2 is strictly
sequential and stops after the first failed call. Thus a failed claim theory,
policy, extractor, or rule suppresses every later scheduled call without any
notion of implementation-chosen “independence.”
R1d has no peer-result propagation: a record is not marked invalid merely
because another R1d record it names is invalid. Cross-record causality,
attempt-to-atom qualification, premise existence, trace consistency, and graph
inventory are R1e predicates and run only when every local record passed R1d.
Within I0b, two unequal available canonical byte strings under one declared
digest emit only `identity.collision` at their collection anchor. Every other
wrong preimage emits `identity.mismatch` at its declared identity field.
At R2 comparison, a claim canonicalization mismatch uses
`relation.claim_noncanonical`; every other required-output mismatch uses
`relation.semantic_mismatch`.

R1 code ownership is fixed:

| Substage | Codes it may emit |
|---|---|
| R1a | `relation.kind_mismatch` |
| R1b | `relation.invalid_binding` |
| R1c | `relation.unauthorized`, `relation.invalid_assumption` |
| R1d | `relation.invalid_decision`, `relation.invalid_attempt`, `relation.invalid_accounting`, `relation.invalid_atom`, `relation.invalid_derivation` |
| R1e | `relation.invalid_trace`, `relation.invalid_graph`, plus `relation.invalid_atom` or `relation.invalid_derivation` for a failed cross-record qualification |
| R1f | `relation.cycle`, `resource.basis_exceeded` |

If several peer predicates in one listed substage fail, all of their anchored
records are emitted. No other relational code is substituted.

## Parse and outer-boundary errors

| Code | Fixed message |
|---|---|
| `package.syntax` | `package source is not valid candidate JSON` |
| `package.duplicate_name` | `package source contains a duplicate object name` |
| `resource.outer_exceeded` | `candidate outer resource limit exceeded` |

## Selta schema errors

| Code | Fixed message |
|---|---|
| `package.shape` | `package wrapper does not match its Selta contract` |
| `package.environment_mismatch` | `package environment does not match the admitted environment` |
| `package.unknown_contract` | `typed document names an unknown contract` |
| `package.document_shape` | `typed document value does not match its Selta contract` |
| `package.verification_error` | `Selta contract evaluation did not complete cleanly` |

`package.document_shape` represents a clean Selta `fail`.
`package.verification_error` represents a non-empty V1 `Report.errors` or an
unexpected non-pass/non-fail evaluation condition. Neither is interpreted as a
claim refutation.
`package.verification_error` is a defensive operational outcome outside the
mathematical conforming-builtin domain; it keeps `assess` total if the embedded
stable verifier reports a host/runtime fault.

## Identity and closure errors

| Code | Fixed message |
|---|---|
| `identity.noncanonical` | `value is not in canonical candidate form` |
| `identity.mismatch` | `declared identity does not match canonical content` |
| `identity.collision` | `one digest names unequal available canonical bytes` |
| `identity.duplicate` | `canonical set contains a duplicate identity` |
| `identity.unknown_semantics` | `semantic binding names unknown semantics` |
| `package.missing_reference` | `tagged document reference cannot be resolved` |
| `package.contract_mismatch` | `tagged document reference has the wrong target contract` |
| `package.reference_cycle` | `tagged document references form a cycle` |
| `package.unreachable_document` | `package contains a document unreachable from its root` |

`identity.noncanonical` covers set order, sequence rules fixed by the DSL,
unsafe integers, and noncanonical embedded identity preimages after structural
admission has succeeded. An unsafe integer is anchored at its exact occurrence
in the conceptual input workspace.
`identity.collision` has phase `identity` for admitted input, `interpret` for
generated-state insertion, and `output` for concluded-outcome insertion.

## Relational errors

| Code | Fixed message |
|---|---|
| `basis.mismatch` | `package root does not match the caller-pinned assessment basis` |
| `relation.unauthorized` | `assessment basis does not grant the required exact role` |
| `relation.invalid_binding` | `semantic binding is invalid at this call site` |
| `relation.kind_mismatch` | `semantic descriptor kind is invalid at this call site` |
| `relation.semantic_mismatch` | `semantic output differs from required output` |
| `relation.claim_noncanonical` | `claim theory did not return the submitted canonical claim` |
| `relation.invalid_assumption` | `derivation or interpretation names an unavailable assumption` |
| `relation.invalid_decision` | `acquisition decision is invalid` |
| `relation.invalid_attempt` | `acquisition attempt relation or disposition is invalid` |
| `relation.invalid_trace` | `acquisition trace is causally or relationally invalid` |
| `relation.invalid_accounting` | `scoped accounting attestation is invalid` |
| `relation.invalid_graph` | `evidence graph inventory or relation is invalid` |
| `relation.invalid_atom` | `evidence atom is invalid` |
| `relation.invalid_derivation` | `evidence derivation is invalid` |
| `relation.cycle` | `evidence premise graph contains a cycle` |
| `resource.basis_exceeded` | `assessment basis resource limit exceeded` |

Substage precedence is exhaustive. For example, a wrong decision digest stops
in I0 as `identity.mismatch`; only a correctly identified decision reaches R1d
and can emit `relation.invalid_decision`. A wrong-kind binding stops in R1a, an
invalid parameter binding in R1b, and missing authority in R1c; none also emits
an enclosing node or trace error. A claim-theory canonicalization input that
fails its contract or call domain emits `relation.invalid_binding` in R1b at
the claim's complete theory binding and does not dispatch.

`resource.basis_exceeded` uses the phase in which the comparison first becomes
available: `relation` for input/graph counters; the phase of the denied dispatch
for `max_semantic_calls`; `interpret` when generated state changes document,
byte, or state-byte counts; and `project` for conclusion-label count. Its
pointer always anchors the exact limiting `max_*` field in the input basis.

## Interpretation errors

| Code | Fixed message |
|---|---|
| `evaluation.invalid_output` | `semantic relation returned an invalid typed output` |
| `evaluation.fuel_exhausted` | `semantic fuel limit exceeded` |
| `evaluation.arithmetic_overflow` | `candidate resource arithmetic overflowed SafeInt` |

These codes use the phase of the call being evaluated. A submitted typed result
that differs from a re-executed relation is `relation.semantic_mismatch`. A
generated state or projection result that fails its sole output contract is
`evaluation.invalid_output`. A contract-valid but specification-wrong builtin
is a nonconforming implementation exposed by the oracle corpus; the runtime
does not pretend to prove its own implementation correct by running the same
code twice.
`evaluation.invalid_output` is likewise a defensive outcome for a supplied
implementation that violated its host-acceptance precondition, not an admitted
epistemic state.

## Projection and output errors

| Code | Fixed message |
|---|---|
| `output.too_large` | `candidate outcome exceeds the outer output limit` |
| `error.truncated` | `additional errors were omitted` |

## Exact pointer anchors

Let `doc(i)` be `/input/documents/<i>/value` in the submitted canonical document
array. An *object anchor* is the pointer to the whole object, not a selected
member. The following anchors are exhaustive:

| Failure | Exact path |
|---|---|
| package wrapper shape or verification | `/input` |
| environment mismatch | `/input/environment` |
| unknown document contract | `/input/documents/<i>/contract` |
| document value fail or verification error | `/input/documents/<i>/value` |
| noncanonical collection | its `/input/...` array pointer |
| unsafe integer | its exact `/input/...` integer occurrence |
| duplicate collection member | its `/input/...` array pointer; one record regardless of duplicate count |
| typed-document digest mismatch | `/input/documents/<i>/digest` |
| embedded identity mismatch | its exact `/input/.../id`, `plan_id`, or digest member |
| digest collision between document entries | `/input/documents` |
| digest collision between decisions | the containing `/input/.../decisions` array |
| digest collision between attempt plan or completed IDs | the containing `/input/.../attempts` array |
| digest collision between grants | `doc(basis)/trust/grants` |
| digest collision between assumptions | `doc(basis)/assumptions` |
| digest collision between evidence nodes | `doc(graph)/nodes` |
| unknown semantics | the exact `/input/.../semantics` member |
| missing or wrong-contract reference | the complete `/input/...` `DocumentRef` occurrence |
| document-reference cycle | `/input/documents`; one record regardless of cycle count |
| unreachable document | `/input/documents/<i>`; one record per unreachable entry |
| caller basis mismatch | `/input/root` |
| invalid or wrong-kind semantic binding | the complete `/input/...` binding object |
| unauthorized semantic role | the complete `/input/...` binding object |
| unauthorized source role | the atom's complete `/input/.../attempt` object |
| unauthorized recorder role | the complete `/input/...` scoped-accounting object |
| unavailable assumption | that exact `/input/...` assumption-ID occurrence in a derivation or binding parameters |
| invalid decision, attempt, accounting, atom, or derivation | that complete `/input/...` record object |
| invalid graph inventory or graph-wide relation | `doc(graph)` |
| invalid trace relation spanning records | the complete `/input/.../trace` object |
| graph premise cycle | `doc(graph)/nodes`; one record regardless of cycle count |
| noncanonical claim result | `doc(claim)` |
| submitted decision semantic mismatch | that decision's `/input/.../output` reference |
| rejected scoped-attestation verifier result | the `/input/.../attestation` reference |
| submitted atom or derivation semantic mismatch | the complete `/input/...` node object |
| contract-invalid semantic output | the same claim, decision, attestation, atom, or derivation anchor used for its semantic mismatch |
| basis resource exceeded | pointer to `doc(basis)/limits/<matching max_* field>` |
| generated interpreter output invalid | `/generated/state` |
| generated projection output invalid | `/generated/conclusions` |
| generated state digest collision | `/generated/state` |
| concluded outcome digest collision | `/generated` |
| semantic call or fuel overflow | `/generated/resources/semantic_calls` or `/generated/resources/semantic_fuel` respectively |
| another resource arithmetic overflow | its exact `/generated/resources/<counter>` |
| output too large | `/generated` |
| truncation marker | empty pointer |

`doc(graph)`, `doc(claim)`, and `doc(basis)` mean the actual resolved document
indices, not literal pointer tokens. A bad source that creates several invalid
records receives one error per anchored record unless the table fixes a single
collection-level error. When several codes share one anchor and phase, lexical
code order is required. Implementations do not choose a friendlier anchor,
message, or multiplicity.

Collision anchoring first uses identity containment. If both unequal preimages
belong to one canonical collection, use that collection's array pointer. If
they belong to distinct collections in one `TypedDocument`, use `doc(i)`. If
they belong to different typed documents, including acquisition-manifest or
cross-acquisition decision/attempt collisions, use `/input/documents`. The
class-specific collision rows above are the common same-collection cases.
Generated-state and concluded-outcome comparisons instead use their explicit
`/generated` anchors and the phase in which insertion would occur.

## Error package and charged work

An error package contains exactly one typed document: its error outcome. It
never retains the submitted basis or any partially admitted input document.
The `environment` field is the already admitted environment ID.

After ordinary outcome construction, measure
`UTF8_bytes(JCS(TypedDocument<AssessmentOutcome>))`. If it exceeds 1,048,576
bytes, replace the outcome with the fixed minimal error form: the same
environment and actual resource counters, empty `executions`, and exactly one
`output.too_large` error at `/generated`. The replacement is then hashed and
wrapped as the sole package document. This rule applies to an oversized
concluded outcome and to an oversized ordinary error outcome; it cannot recurse
because the replacement has fixed-size fields and eleven SafeInt counters.

Resource counters commit at these exact boundaries:

- before S0 succeeds, every counter is zero;
- after S0, `documents` is the number of `TypedDocument` array entries and
  `total_document_bytes` is the checked sum of
  `UTF8_bytes(JCS(TypedDocument))` over every entry, including entries later
  rejected or found duplicate;
- after identity phase succeeds, `attempts_total` is the sum over acquisitions
  declared in `graph.acquisitions`; node, edge, and fan-in counters use the
  structurally present `graph.nodes` and their premise arrays, even when R1
  later rejects an inventory relation;
- `graph_depth` uses only evidence-premise edges whose target node exists in
  `graph.nodes`; it is zero if any such target is missing or if those edges are
  cyclic, while the other structural graph counters retain their values;
- semantic calls and fuel are the calls actually dispatched under the fixed
  schedule, including a call whose charged output crosses the fuel limit;
- after a contract-valid state value is wrapped, `documents` increases by one,
  its typed-document bytes enter `total_document_bytes`, and `state_bytes`
  commits;
  and
- `conclusion_labels_total` commits only after a contract-valid projection
  value is produced.

A counter whose commit boundary was not reached is zero. `executions` is the
canonical set, by semantics ID, of implementations actually dispatched before
the stop; repeated calls under one semantics ID yield one record because
`Sigma` binds one implementation for the assessment. The outcome document
itself is never charged.

After state counters commit, compare `max_documents`,
`max_total_document_bytes`, and `max_state_bytes` as one interpret substage and
emit one `resource.basis_exceeded` record for every exceeded field. Static R1f
likewise emits one record for every exceeded input/graph limit. Project compares
only `max_conclusion_labels_total`. Sorting by the exact basis-field pointers,
not discovery order, fixes the returned byte order.
