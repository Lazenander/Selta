# 23 — Positive-controls conformance profile

> **Status: S2 normative conformance-profile candidate.** This document is
> the complete semantic specification for `positive-controls-candidate-1`.
> It demonstrates one constructive witness rule and one recoverability-only
> sequential basis. It is test evidence, not an application policy, an
> authentication mechanism, or a statistical assurance claim.

## Purpose and boundary

The negative cases in document 14 do not imply that every amplification path
must fail. This profile represents its two positive controls without adding an
assurance order to the candidate core:

- `CHECKED-WITNESS` introduces one acquired witness through a named checker
  under one exact caller-pinned soundness assumption; and
- `CALIBRATED-SEQUENTIAL-EVIDENCE` establishes only that seven required
  declarations are caller-bound and recoverable from one dependency closure.

The profile is an artifact-level extension of `mechanics-candidate-1`. Its
environment reuses the complete mechanics environment's contracts and
semantic descriptors by exact identity and adds only the contracts and three
descriptors specified here. Existing schema sources are resolved from their
existing artifact trees and are not copied.

The three added descriptors are deliberately separate authorities:

```text
checked existential witness       rule
sequential accounting expectation attestation
sequential basis recovery         rule
```

One semantic grant therefore cannot authorize both witness introduction and
sequential-basis recovery. The profile changes no candidate core schema,
identity rule, phase, error, resource schedule, interpreter, or projection.

## Schema inventory

The profile adds exactly these eight closed Selta schema sources:

```text
sha256-preimage-existential.schema.json
checked-witness-theory.schema.json
checked-witness-rule-input.schema.json
sequential-assumption.schema.json
sequential-basis.schema.json
sequential-accounting-expectation.schema.json
sequential-attestation-input.schema.json
sequential-recovery-rule-input.schema.json
```

It reuses the mechanics `EvidenceResult` and `AttestationResult` contracts and
the presence `Text` contract. Every occurrence of a digest, document
reference, semantic binding, atom, accounting record, and fact reference has
the corresponding candidate-1 shape. No schema in this profile admits an open
object or an untyped value.

## Checked existential witness

### Proposition and theory

`Sha256PreimageExistential` is:

```text
{
  revision: "selta.evidence.sha256-preimage-existential/candidate-1",
  target: Digest
}
```

It denotes the proposition:

```text
exists text w. SHA256(UTF8(w)) = target
```

`UTF8(w)` is the exact UTF-8 encoding of the decoded JSON string value. JSON
quotes and escapes are not part of the preimage, and Unicode is not
normalized.

`CheckedWitnessTheory` is:

```text
{
  revision: "selta.evidence.checked-witness-theory/candidate-1",
  claim: DocumentRef<Claim>,
  proposition: DocumentRef<Sha256PreimageExistential>,
  modality: DocumentRef<Text>
}
```

The rule binding's exact parameter document is also the sole consumed
soundness assumption's `statement`. That assumption's `scope` is the exact
`claim` reference. The semantic grant authorizes the rule to run; the
assumption records the caller-accepted soundness premise. Neither implies the
other.

### Rule input and descriptor

`CheckedWitnessRuleInput` is:

```text
{
  parameters: CheckedWitnessTheory,
  premises: [{
    fact: { evidence: Digest },
    resolved: { evidence: Atom }
  }],
  assumptions: [Digest; exactly 1],
  arguments: {
    reference: DocumentRef<Text>,
    value: Text
  }
}
```

The descriptor is:

| Field | Value |
|---|---|
| kind | `rule` |
| parameter contract | `CheckedWitnessTheory` |
| input contract | `CheckedWitnessRuleInput` |
| output contract | reused `EvidenceResult` |
| premise order | `set` |
| execution class | `builtin_total` |

The assessor constructs `resolved.evidence` from the sole `EvidenceFact` and
constructs `arguments` from the derivation's argument reference. The rule
receives no general package resolver.

### Exact predicates and relation

R1b owns the binding predicates:

1. `parameters.claim` resolves under the core Claim contract and uses the
   reused exact claim theory;
2. `parameters.proposition` resolves under the
   `Sha256PreimageExistential` contract;
3. the claim's proposition reference equals `parameters.proposition`;
4. `parameters.modality` resolves under the presence Text contract with the
   exact value `checked-witness`; and
5. every parameter reference has the contract required above.

A failure is `relation.invalid_binding` at the complete rule binding and
causes no dispatch.

R1c requires the exact checker semantic grant and availability of the one
listed assumption. A missing grant is `relation.unauthorized`; an unavailable
assumption is `relation.invalid_assumption`.

R1d owns the local derivation and submitted-input predicates:

1. the listed assumption's statement reference equals the binding's parameter
   reference and its scope equals `parameters.claim`;
2. `arguments.reference` equals the submitted derivation argument and resolves
   under the presence Text contract to `arguments.value`; and
3. lowercase hexadecimal SHA-256 of `UTF8(arguments.value)`, displayed with
   the `sha256:` prefix, equals the proposition's `target`.

A failure is `relation.invalid_derivation` at the complete derivation and
causes no dispatch.

R1e owns the cross-record premise qualification:

1. the sole fact resolves to the exact atom copied into
   `resolved.evidence`;
2. that atom uses the mechanics exact-observation extractor;
3. its stance is `support`;
4. its qualified completed attempt is `observed + include`; and
5. the atom payload, observed-response reference, and
   `arguments.reference` are equal.

A failure is `relation.invalid_derivation` at the complete derivation.

On the admitted call domain the relation returns:

```text
{
  claim: parameters.claim,
  stance: "support",
  modality: parameters.modality,
  payload: arguments.reference
}
```

R2 compares that value with the submitted derivation. A different claim,
stance, modality, or payload is `relation.semantic_mismatch` under the generic
schedule.

### Invalid witnesses do not refute existence

A witness whose hash does not match the proposition fails R1d and creates no
evidence result. A valid package that omits such a failed derivation leaves the
existential target at `neither`. A submitted refuting derivation cannot match
the rule's support-only output. Failure to exhibit one witness is never
converted into evidence that no witness exists.

## Recoverable sequential basis

### Seven exact roles

`SequentialBasis` is:

```text
{
  revision: "selta.evidence.sequential-basis/candidate-1",
  scope: DocumentRef,
  population: Digest,
  frame: Digest,
  dependence: Digest,
  calibration: Digest,
  history: Digest,
  statistic: Digest,
  stopping: Digest
}
```

`scope` is an independently constructed reference under the reused non-empty
presence Text contract. Its value is a caller-pinned scope declaration, not
the recoverability claim. The seven IDs MUST be pairwise distinct. Each names
one assumption in the caller-pinned basis whose exact scope equals this
independent reference.
`SequentialAssumption` uses exactly two closed structural variants to encode
the seven semantic roles. The six roles without additional role-specific
structure share one variant; stopping remains separate because it alone
carries a regime:

```text
{
  revision,
  role: "population" | "frame" | "dependence" |
        "calibration" | "history" | "statistic",
  declaration: DocumentRef
}

{ revision, role: "stopping",
  regime: "fixed_horizon" | "anytime_valid",
  declaration: DocumentRef }
```

Both structural variants use revision
`selta.evidence.sequential-assumption/candidate-1`. The independent scope and
declaration references remain typed and recoverable but opaque to this
profile. No relation here tests their truth, calibration, stochastic validity,
or empirical adequacy.

### Identity dependency order

The sequential construction has this strict topological identity order:

```text
declarations and independent Text scope
  -> SequentialAssumption statement documents
  -> seven Assumption IDs
  -> SequentialBasis
  -> recoverability meta-claim
  -> semantic parameter, acquisition, graph, and assessment basis
```

The assumption IDs bind their statement and independent scope. The
`SequentialBasis` binds those seven bare IDs and that same scope, but it does
not reference the recoverability claim. The claim then references the
`SequentialBasis` as its proposition. There is therefore no edge from the
basis or any assumption back to the claim. Core document-cycle admission
continues to reject any application document that attempts to introduce a
tagged-reference back-edge. In the admitted profile construction the scope is
a Text scalar and therefore cannot contain such an edge at all.

### Scoped accounting expectation

`SequentialAccountingExpectation` is:

```text
{
  revision:
    "selta.evidence.sequential-accounting-expectation/candidate-1",
  result: AttestationResult,
  recorder: DocumentRef,
  scope: DocumentRef,
  checkpoint: DocumentRef,
  exceptions: [DocumentRef],
  assertion: DocumentRef<SequentialBasis>
}
```

`SequentialAttestationInput` contains that resolved parameter value followed
by the ordinary scoped record's recorder, scope, checkpoint, exceptions, and
attestation fields. Its descriptor is:

| Field | Value |
|---|---|
| kind | `attestation` |
| parameter contract | `SequentialAccountingExpectation` |
| input contract | `SequentialAttestationInput` |
| output contract | reused `AttestationResult` |
| execution class | `builtin_total` |

R1b verifies the parameter contract, resolves `assertion` under the
`SequentialBasis` contract, resolves that basis's `scope` under the reused
non-empty presence Text contract, and requires `parameters.scope` to equal
that independent scope reference. A mismatch is `relation.invalid_binding` at
the complete verifier binding and causes no dispatch. R1c requires the exact
semantic and recorder grants fixed by the core.

R1d requires exact equality between the expectation and scoped record:

```text
input.recorder    = parameters.recorder
input.scope       = parameters.scope
input.checkpoint  = parameters.checkpoint
input.exceptions  = parameters.exceptions
input.attestation = parameters.assertion
```

Any mismatch is `relation.invalid_accounting` at the complete scoped
accounting object and causes no dispatch. On the admitted domain the relation
returns `parameters.result`. A scoped record still requires the exact accepted
result; a contract-valid rejected result therefore has the ordinary R2
`relation.semantic_mismatch` behavior.

This relation authenticates nothing and proves no trace completeness. It only
checks that one submitted scoped record matches one caller-selected typed
expectation.

### Recovery rule

The recovery rule parameter contract is the reused `EvidenceResult`. Its value
is restricted to:

```text
{
  claim: <recoverability meta-claim>,
  stance: "support",
  modality: <Text with value "sequential-basis-recoverability">,
  payload: <DocumentRef<SequentialBasis>>
}
```

The meta-claim's proposition reference equals that payload. A separate
substantive statistical target receives no evidence from this rule.

`SequentialRecoveryRuleInput` is:

```text
{
  parameters: EvidenceResult,
  premises: [{
    fact: { accounting: DocumentRef<Acquisition> },
    resolved: { accounting: ScopedAccountingAttestation }
  }],
  assumptions: [Digest; exactly 7],
  arguments: {
    reference: DocumentRef<SequentialBasis>,
    value: SequentialBasis
  }
}
```

The descriptor is:

| Field | Value |
|---|---|
| kind | `rule` |
| parameter contract | reused `EvidenceResult` |
| input contract | `SequentialRecoveryRuleInput` |
| output contract | reused `EvidenceResult` |
| premise order | `set` |
| execution class | `builtin_total` |

R1b requires:

1. `parameters.stance` to be `support`;
2. its modality to be presence Text with value
   `sequential-basis-recoverability`;
3. its payload to resolve under the `SequentialBasis` contract;
4. the parameter claim's proposition to equal its payload;
5. the basis's independent `scope` reference to resolve under the reused
   non-empty presence Text contract and therefore remain distinct from the
   recoverability claim; and
6. all seven role IDs in that resolved sequential basis to be pairwise
   distinct.

A failure is `relation.invalid_binding` at the complete rule binding.

R1c requires the recovery-rule grant and availability of all seven listed
assumptions. Missing authority is `relation.unauthorized`; any unavailable ID
is `relation.invalid_assumption`.

R1d requires:

1. the derivation assumptions to equal the canonical set of all seven role
   IDs;
2. each available assumption statement to have the
   `SequentialAssumption` contract and matching role;
3. each assumption scope to equal `arguments.value.scope`; and
4. `arguments.reference` to equal the submitted derivation argument and
   `parameters.payload`, and to resolve under the `SequentialBasis` contract
   exactly to `arguments.value`.

A failure is `relation.invalid_derivation` at the complete derivation.

R1e requires:

1. the sole `AccountingFact` to resolve to the copied scoped accounting value;
2. that scoped record to use the sequential accounting expectation
   descriptor; and
3. its assertion reference to equal `arguments.reference`.

A failure is `relation.invalid_derivation` at the complete derivation. On the
admitted domain, the relation returns `input.parameters` unchanged. R2 performs
the ordinary canonical comparison.

The resulting support concerns only this proposition:

> the exact seven-role basis is caller-bound and dependency-recoverable.

It does not support the truth of a source, assumption, calibration model,
statistic, stopping guarantee, or substantive target.

## Authority and dependency closures

The checked-witness state closure contains the checked derivation, observation
atom, qualified attempt, source grant, acquisition-policy dependencies,
extractor, checker rule, reached claim theories, interpreter, and exact
soundness assumption. The conclusion closure additionally contains the reused
projection binding and grant.

The sequential state closure contains its recovery derivation and
`AccountingFact`, all seven assumptions, acquisition-policy dependencies, the
recorder grant, sequential attestation binding and grant, recovery-rule
binding and grant, reached claim theories, and interpreter. The conclusion
closure additionally contains the reused projection binding and grant. It has
no source grant because an accounting fact does not consume source content.
The independent sequential scope remains recoverable through the basis payload
and every consumed assumption; it is not the recoverability claim.

For both controls, state `direct_inputs` remain exactly the target claim,
assessment basis, and graph roots fixed by the presence interpreter. A
conclusion adds the generated state document. Parameter, proposition, witness,
assertion, checkpoint, exception, assumption statement, scope, and sequential
declaration documents remain transitively recoverable and are not promoted to
additional direct roots.

No result, rule, or attestation may add a grant, assumption, target, semantic
binding, or resource limit to the caller-pinned basis.

## Mandatory conformance cases

`CHECKED-WITNESS` requires:

1. one positive acquired matching witness and checked support derivation;
2. an invalid witness omitted from the graph, leaving the existential target
   at `neither`;
3. an invalid witness submitted as support, producing
   `relation.invalid_derivation`;
4. a valid witness submitted as refutation, producing
   `relation.semantic_mismatch`;
5. missing checker authority, unavailable theory assumption, and available
   wrong-statement or wrong-scope cases; and
6. forbidden closure mutations omitting the theory assumption, observation
   atom, or attempt dependency.

`CALIBRATED-SEQUENTIAL-EVIDENCE` requires:

1. one accepted scoped accounting record whose assertion is the exact
   sequential basis and whose expected scope equals that basis's independent
   scope;
2. seven distinct role assumptions and one recovery derivation over its
   `AccountingFact`;
3. a `support_only` recoverability meta-target beside a substantive target
   that remains `neither`;
4. both `fixed_horizon` and `anytime_valid` stopping variants;
5. a basis scope with the wrong contract and a basis-scope versus
   expectation-scope mismatch at R1b, checkpoint mismatch at R1d, missing
   recorder authority, missing attestation authority, and a rejected-result
   case;
6. unavailable and omitted assumptions, one ID reused across two basis roles,
   and role-, statement-, and scope-mismatched assumption cases;
7. an attempted substantive-assurance binding or output; and
8. one forbidden closure mutation for each of the seven roles and one omitting
   the independent scope.

A checkpoint mismatch between an otherwise binding-valid expectation and
scoped record is the ordinary wire-reachable fixture for
`relation.invalid_accounting` in R1d. A parameter scope unequal to the asserted
basis scope fails earlier as `relation.invalid_binding` in R1b.

These cases establish representability, explicit authority, exact
recoverability, and constructive witness introduction only. They are not
empirical validation of a checker, recorder, stochastic model, calibration
corpus, or sequential method.
