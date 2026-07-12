# 22 — Mechanics conformance profile

> **Status: S2 normative conformance-profile candidate.** This document is the
> complete semantic specification for the acquisition and provenance mechanics
> exercised by `mechanics-candidate-1`. It is isolated test evidence, not an
> application policy, product API, implementation guide, or platform-support
> claim.

## Purpose and boundary

The presence profile in document 19 deliberately contains no acquisition
policy, attestation verifier, extractor, or operational-fact rule. This profile
adds the smallest closed set of total relations needed to exercise those
generic candidate-1 paths without changing the presence profile or assigning
them an application meaning.

The mechanics environment reuses, by exact content identity:

- all seven candidate core contracts from document 16;
- the candidate pure-builtin evaluation profile and its five configuration
  schema sources;
- the presence `unit`, non-empty `text`, label, exact-claim-theory,
  presence-interpreter, presence-state, cautious-projection, and projection
  parameter contracts; and
- the value shape currently named by
  `profiles/evidence/presence-candidate-1/schemas/assumption-rule-output.schema.json`
  as the generic `EvidenceResult` contract.

`EvidenceResult` is exactly:

```text
EvidenceResult {
  claim: DocumentRef<Claim>,
  stance: "support | refute",
  modality: DocumentRef,
  payload: DocumentRef
}
```

The exact-claim-theory, presence-interpreter, and cautious-projection
descriptors are reused unchanged, including their document-19 specification
source. The seven new descriptors below bind the exact bytes of this document
as their semantic-specification source. The mechanics environment does not
contain the presence explicit-assumption rule.

The profile introduces no probability, calibration, voting, reliability,
alias relation, clock, native path, process, transport, or provider concept.
It authenticates no real recorder and recommends no scheduling, selection,
stopping, or missingness policy.

## Schema inventory

The profile adds exactly these nine Selta schema sources under its
conformance-only artifact tree:

```text
policy-input.schema.json
attempt-proposal.schema.json
disposition-proposal.schema.json
stop-transition.schema.json
attestation-input.schema.json
attestation-result.schema.json
extractor-input.schema.json
operational-rule-input.schema.json
forwarding-rule-input.schema.json
```

Every object is closed. Every `Digest`, `DocumentRef`, `Attempt`, `Trace`,
`AccountingAttestation`, `EvidenceNode`, and `FactRef` occurrence has exactly
the candidate-1 shape in document 15 and the corresponding core schema.
Collections use document 15's canonical order. No new schema uses `any` for a
core mechanics value.

### Policy values

`AttemptProposal` is:

```text
AttemptProposal {
  ordinal: SafeInt,
  predecessors: [Digest],
  source: DocumentRef,
  request: DocumentRef,
  context_view: DocumentRef
}
```

It is the attempt-plan preimage of document 16 without `manifest` and
`scheduled_by`. `DispositionProposal` is:

```text
DispositionProposal {
  kind: "include | exclude | censor",
  reason: DocumentRef
}
```

`StopTransition` is exactly:

```text
{ stop: true }
```

`ManifestView` is an inline closed value, not a ninth contract:

```text
ManifestView {
  frame: DocumentRef,
  channel: DocumentRef,
  context: DocumentRef,
  sources: [DocumentRef],
  budget: DocumentRef,
  randomness?: DocumentRef
}
```

`PolicyInput` is one closed union. All three policy descriptors use the same
input contract and the existing `Unit` parameter contract. Its variants are:

```text
ScheduleInput {
  operation: "schedule",
  parameters: {},
  manifest: Digest,
  observed_plans: [Digest],
  manifest_view: ManifestView,
  snapshot: { proposal: AttemptProposal }
}

SelectInput {
  operation: "select",
  parameters: {},
  manifest: Digest,
  observed_plans: [Digest],
  manifest_view: ManifestView,
  snapshot: {
    plan_id: Digest,
    outcome: AttemptOutcome,
    disposition: DispositionProposal
  }
}

StopInput {
  operation: "stop",
  parameters: {},
  manifest: Digest,
  observed_plans: [Digest],
  manifest_view: ManifestView,
  snapshot: { transition: StopTransition }
}
```

Each `Decision.input` MUST resolve under the `PolicyInput` contract. Its value,
not its `DocumentRef`, is the complete semantic invocation supplied to the
policy relation. The assessor adds no wrapper and supplies no document
resolver. In every variant:

- `parameters` equals the resolved policy binding parameter `{}`;
- `manifest` equals the acquisition-manifest digest;
- `observed_plans` equals the decision field byte-for-byte;
- `manifest_view` equals the containing manifest after removal of its three
  policy bindings and no other change; and
- `operation` equals the decision kind.

For `select`, `snapshot.plan_id` identifies the one attempt selected by the
decision and `snapshot.outcome` equals that completed attempt's outcome. Thus
selection binds the outcome before dispatch without a hidden read. For
`schedule`, the proposal is the value later bound into the attempt plan. For
`stop`, the only transition is `{ "stop": true }`.

Each `Decision.output` resolves under the corresponding scalar output
contract: `AttemptProposal`, `DispositionProposal`, or `StopTransition`.

### Attestation values

`AttestationResult` is the closed union:

```text
{ status: "accepted" }
{ status: "rejected" }
```

`AttestationInput` is:

```text
AttestationInput {
  parameters: AttestationResult,
  recorder: DocumentRef,
  scope: DocumentRef,
  checkpoint: DocumentRef,
  exceptions: [DocumentRef],
  attestation: DocumentRef
}
```

The descriptor uses `AttestationResult` as both its parameter and output
contract. `Scoped.attestation` is an opaque assertion input. It is not a
submitted verifier result and its document is never replaced by one. The
assessor constructs `AttestationInput` directly from the scoped accounting
record and the resolved parameter value; the relation receives no resolver.

The expected successful verifier result is exactly
`{ "status": "accepted" }`. A contract-valid `rejected` result is compared
against that expected result and produces `relation.semantic_mismatch` at the
scoped record's complete `attestation` reference. The call, output fuel, and
execution record commit before that mismatch under document 16's R2 schedule.
No verifier-result document is stored or charged.

This conformance relation does not authenticate the assertion. Its parameter
merely selects the deterministic accepted or rejected control result needed to
exercise the generic accounting boundary.

### Extractor values

`ExtractorInput` is:

```text
ExtractorInput {
  parameters: EvidenceResult,
  attempt: AttemptRef,
  response: {
    reference: DocumentRef<Text>,
    value: Text
  }
}
```

The descriptor uses `EvidenceResult` as its parameter and output contract. The
assessor constructs `response` before dispatch. Its reference equals the
qualified attempt's `Observed.response`, and its value is that document's
resolved non-empty text value. The attempt MUST be `observed + include`, and
the exact source grant required by document 16 MUST exist before construction.
The extractor receives no resolver and cannot read another document.

### Rule values and shallow fact resolution

`OperationalRuleInput` and `ForwardingRuleInput` share this envelope:

```text
RuleInputEnvelope {
  parameters: EvidenceResult,
  premises: [ResolvedPremise; exactly 1],
  assumptions: [],
  arguments: DocumentRef
}
```

Each contract admits only the premise variants its authority requires.
`ForwardingRuleInput` admits the `EvidenceFact` form; `OperationalRuleInput`
admits the other three forms. Every `ResolvedPremise` retains the original fact
and exactly one shallow projection:

```text
{ fact: { evidence: Digest },
  resolved: { evidence: EvidenceNode } }

{ fact: { attempt: AttemptRef },
  resolved: { attempt: Attempt } }

{ fact: { trace: DocumentRef<Acquisition> },
  resolved: { trace: Trace } }

{ fact: { accounting: DocumentRef<Acquisition> },
  resolved: { accounting: AccountingAttestation } }
```

The `EvidenceNode`, `Attempt`, `Trace`, and `AccountingAttestation` values are
complete values from the already admitted graph or acquisition. Shallow
resolution copies those values but does not follow any nested `DocumentRef`.
In particular, an `Observed.response`, `Malformed.raw`, `Malformed.error`,
`Unavailable.reason`, disposition reason, decision input/output, assertion,
exception, request, source, context view, parameter, modality, payload, or
arguments reference remains only its tagged identity.

The assessor constructs the complete `ResolvedPremise` before dispatch. No
mechanics rule receives `D`, a callback, or a resolver. Document 21's paired
semantic-read and forbidden-read controls inject one deliberate boundary probe
into the conformance harness; a trap proves that the probe cannot obtain the
value. Neither control is an input available to a conforming relation.

The two rule descriptors use `EvidenceResult` as their parameter and output
contracts and `premise_order: "set"`; their distinct input contracts preserve
separate semantic identities and grants. Granting evidence forwarding does not
grant operational-fact conversion. With exactly one premise, set and sequence
traversal coincide, but the declared set signature remains part of each
descriptor ID.

## Seven mechanics relations

All seven descriptors use `execution_class: "builtin_total"`. The table is
the complete descriptor-role inventory; exact IDs are computed only after the
schema sources and this document are frozen.

| Role | Kind | Parameter contract | Input contract | Output contract |
|---|---|---|---|---|
| declared proposal | `schedule` | presence `Unit` | `PolicyInput` | `AttemptProposal` |
| declared disposition | `selection` | presence `Unit` | `PolicyInput` | `DispositionProposal` |
| deterministic stop | `stopping` | presence `Unit` | `PolicyInput` | `StopTransition` |
| accounting control | `attestation` | `AttestationResult` | `AttestationInput` | `AttestationResult` |
| exact observation | `extractor` | presence `EvidenceResult` | `ExtractorInput` | presence `EvidenceResult` |
| explicit operational fact | `rule` | presence `EvidenceResult` | `OperationalRuleInput` | presence `EvidenceResult` |
| one-premise forwarding | `rule` | presence `EvidenceResult` | `ForwardingRuleInput` | presence `EvidenceResult` |

### Declared proposal

Call domain:

- `operation` is `schedule`;
- the common policy equalities above hold;
- `snapshot.proposal.predecessors` is canonical; and
- the decision output has the `AttemptProposal` contract.

The relation returns `input.snapshot.proposal` unchanged. R1e separately
requires that proposal to equal the referenced attempt's ordinal,
predecessors, source, request, and context view and that the decision's
`observed_plans` equals `pred*(p)`.

### Declared disposition

Call domain:

- `operation` is `select`;
- the common policy equalities above hold;
- the snapshot plan and outcome equal the selected completed attempt; and
- the decision output has the `DispositionProposal` contract.

The relation returns `input.snapshot.disposition` unchanged. R1e separately
requires that result to equal the attempt's disposition kind and reason, that
`observed_plans` equals `pred*(p) union {p}`. The legality of the resulting
outcome/disposition pair is the local Attempt check in R1d.

### Deterministic stop

Call domain:

- `operation` is `stop`;
- the common policy equalities above hold;
- `snapshot.transition` is `{ "stop": true }`; and
- the decision output has the `StopTransition` contract.

The relation returns `{ "stop": true }`. R1e separately requires
`observed_plans` to equal the complete plan-ID set. This includes the empty set
for a zero-attempt acquisition.

### Accounting control

The call domain requires canonical `exceptions`, exact equality between the
input and scoped record, and an exact parameter value under
`AttestationResult`. The relation returns `input.parameters` unchanged. An
accepted result admits the accounting transition; a rejected result has the
R2 behavior fixed above. The assertion and every other nested reference remain
opaque.

### Exact observation

The call domain requires:

- exact equality of the qualified attempt;
- an existing `observed + include` completed attempt;
- exact response reference and resolved text equality;
- the parameter claim to equal the atom claim; and
- the parameter result's claim, stance, modality, and payload references to
  resolve under the contracts required at the atom call site.

The relation returns `input.parameters` unchanged. Canonical comparison with
the submitted atom therefore checks all four evidence-result fields.

### Explicit operational fact

The call domain requires the sole fact to be `AttemptFact`, `TraceFact`, or
`AccountingFact`; its shallow projection must be the exact corresponding
admitted value; and `assumptions` must be empty. The output parameter claim is
independent of any claim mentioned by the operational record. The relation
returns `input.parameters` unchanged.

This is an explicit typed conversion under caller-granted rule authority. It
does not make timeout, malformed data, exclusion, censoring, or missingness an
ambient refutation. It consumes no source content and requires no source grant.

### One-premise forwarding

The call domain requires one `EvidenceFact`, its exact complete evidence-node
projection, and no assumption. `input.parameters` MUST equal that premise
node's `{ claim, stance, modality, payload }` projection. The relation returns
that value unchanged. The submitted derivation may therefore preserve one
evidence result while acquiring its own content identity and complete
recursive provenance; it cannot alter polarity or modality.

## Failure ownership and dispatch

The generic phase and substage schedule in documents 16 and 20 remains
authoritative. For this profile:

- a wrong descriptor kind is `relation.kind_mismatch` in R1a;
- a parameter-contract or profile binding predicate failure is
  `relation.invalid_binding` in R1b;
- a missing semantic, source, or recorder grant is `relation.unauthorized` in
  R1c;
- a malformed local policy snapshot or scoped accounting record is
  respectively `relation.invalid_decision` or
  `relation.invalid_accounting` in R1d; an illegal outcome/disposition pair is
  likewise a local `relation.invalid_attempt` in R1d;
- schedule/selection snapshot equality with a peer attempt, trace causality,
  atom-to-attempt qualification, graph inventory, and resolved-premise
  qualification are cross-record R1e checks with the exact code ownership in
  document 20; assessor-constructed extractor and resolved-rule invocations
  are dispatched only after those checks succeed; and
- after all prechecks, a submitted policy, atom, or derivation result unequal
  to the relation output is `relation.semantic_mismatch` in R2.

The accepted/rejected accounting comparison is the only mechanics result not
stored as a submitted output document, and its exact R2 mismatch anchor is
fixed above. A contract-invalid injected output remains
`evaluation.invalid_output`. Call limit, fuel, overflow, execution recording,
and stop-on-first-failed-call behavior are unchanged.

## Resource schedule

Every dispatch costs exactly:

```text
1 + UTF8_bytes(JCS({ "binding": binding, "input": input }))
  + UTF8_bytes(JCS(output))
```

The complete resolved response enters extractor fuel. The complete shallow
resolved premise enters rule fuel. Nested referenced document values do not.
Policy, attestation, extractor, and rule construction performs no semantic
call and adds no separate fuel. Each decision schedules one policy call, each
scoped accounting record one attestation call, and each evidence node one
extractor or rule call exactly as document 16 requires.

No mechanics relation creates a typed document. Only the reused presence
interpreter creates the state document. Dependency-closure construction makes
no semantic call and consumes no semantic fuel.

## Dependency closures

The reused presence interpreter and cautious projection retain the exact
state and conclusion dependency functions in document 19. Their direct input
roots do not change: each state component names the target claim, basis, and
graph; each conclusion additionally names the generated state document.

Recursive closure follows document 16 with these mechanics consequences:

- a reached atom records its `EvidenceFact`, qualified `AttemptFact`, source
  grant, extractor binding and grant, all three acquisition-policy bindings and
  grants, and any scoped recorder/attestation dependencies;
- a reached operational derivation records its own `EvidenceFact`, original
  operational `FactRef`, rule binding and grant, policy dependencies, and any
  applicable recorder/attestation dependencies, but no source grant;
- a reached forwarding derivation records its own `EvidenceFact`, the premise
  `EvidenceFact`, its rule binding and grant, and then the premise's complete
  recursive closure;
- scoped accounting records add the recorder grant plus attestation binding
  and semantic grant; unattested records add neither;
- policy parameter, decision input/output, manifest, attempt, assertion,
  reason, and other nested documents remain recoverable through tagged
  references and are not promoted to additional `direct_inputs`; and
- no closure field records a dispatch-time read trace.

Thus changing an opaque nested response or malformed-raw document changes the
induced document, attempt, acquisition, fact, graph, and basis identities, but
does not authorize an operational rule to inspect that value.

## Mandatory conformance cases

The mechanics corpus is incomplete unless it contains all of the following as
closed conformance cases with exact expected outcomes or errors. Positive and
relation-phase inputs are schema-admitted and identity-closed; a deliberate
schema-boundary case stops at the exact earlier boundary it tests.

1. a zero-attempt acquisition with one stop decision over the empty plan set;
2. one `observed + include` attempt feeding one atom;
3. the same atom without its exact source grant;
4. one `unavailable + exclude` attempt consumed by the operational-fact rule
   for a separate service-health claim while the semantic target remains
   `neither`;
5. an invalid atom over that unavailable attempt;
6. one two-attempt acquisition whose second plan names the first as a
   predecessor, retaining included support and excluded refutation without
   converting the excluded observation into an atom;
7. positive cases for all legal combinations—`observed + include`,
   `observed + exclude`, `observed + censor`, `malformed + exclude`,
   `malformed + censor`, and `unavailable + exclude`—plus
   `malformed + include` and both forbidden unavailable dispositions;
8. accepted scoped accounting, missing recorder authority, missing attestation
   semantic authority, and a contract-valid rejected attestation result;
9. one forwarding derivation over an `EvidenceFact` whose closure reaches an
   atom and its acquisition;
10. exact complete policy, source, recorder, attestation, atom, operational,
    and recursive dependency closures;
11. a metamorphic `observed + exclude` or `malformed + exclude` pair whose
    nested content changes and therefore changes the induced opaque identities
    while the operational-rule result stays equal; and
12. a conformance-only semantic-read attempt paired with a forbidden-read trap,
    and a schema-valid payload-dependent rule output for that metamorphic pair.

Policy cases additionally cover a wrong manifest view, wrong observed-plan
set, selection snapshot with the wrong plan or outcome, noncanonical
predecessors, and a schema-invalid stop transition at its exact schema phase.

These cases establish only deterministic mechanics, authority boundaries,
provenance, resource behavior, and operational separation. They are not
evidence that any acquisition or accounting policy is empirically sound.

## Non-product and privacy boundary

The profile exists only as S2 conformance input. It exposes no stable library,
daemon, host protocol, application entry point, scheduler, storage format, or
migration promise. A future product profile must define and validate its own
policies rather than reuse these echo/control relations.

An extractor receives the resolved authorized response and therefore may
observe its content. Operational rules receive only shallow values containing
nested reference identities. Content hashes remain public equality tests under
document 16's privacy boundary. This profile adds no secrecy, encryption,
authentication, or completeness claim.
