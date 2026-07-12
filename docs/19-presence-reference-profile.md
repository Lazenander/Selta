# 19 — Presence reference profile

> **Status: S2 normative profile candidate.** This profile gives the generic
> evidence language one deliberately weak executable meaning. It introduces no
> probability, voting, reliability, alias equivalence, or assurance scale.

## Artifact boundary

The immutable profile lives under
`profiles/evidence/presence-candidate-1/`. Its environment manifest binds every
schema, the generic language/semantics/error sources, and the profile semantic
specification by the candidate identities in document 16.
The profile contains exactly four evidence semantics:

```text
exact claim canonicalization
explicit-assumption derivation
presence interpretation
cautious conclusion projection
```

It contains no acquisition policy, extractor, attestation verifier, or lineage
normalizer. The one derivation rule exists only to construct honest
caller-approved support/refute conformance cases without inventing an
observation. Other semantics can be added to another environment without
changing the four relations here.

Every operation input and output is an ephemeral typed value under the named
profile schema. Document references resolve only within the admitted package
map `D`. The assessor adds only the generated state identity: under document
20's T1 rule it inserts a new typed document or reuses a byte-equal existing
entry. Labels and parameters are ordinary input documents reachable from the
basis bindings.

## Exact claim theory

Schemas:

- `schemas/unit.schema.json`
- `schemas/claim-theory-input.schema.json`
- `schemas/claim-theory-output.schema.json`

The parameter value is `{}`. The only operation is:

```text
input = {
  operation: "canonicalize",
  parameters: {},
  subject: DocumentRef,
  proposition: DocumentRef,
  boundary: DocumentRef
}

output = {
  subject: input.subject,
  proposition: input.proposition,
  boundary: input.boundary
}
```

The relation resolves all three references and returns them unchanged. Equality
is byte equality of the output triple. It has no incompatibility or negation
operation. The relation is deterministic, idempotent, and contract-preserving.

Whenever a dependency closure reaches a claim under this theory, the assessor
records the exact binding and semantic grant. The subject, proposition, and
boundary remain recoverable through the reached claim reference; the relation
does not emit a dependency witness.

## Explicit-assumption rule

Schemas:

- `schemas/assumption-rule-parameters.schema.json`
- `schemas/assumption-rule-input.schema.json`
- `schemas/assumption-rule-output.schema.json`

The parameter value is:

```text
{
  assumption: Digest,
  claim: DocumentRef<Claim>,
  stance: "support | refute",
  modality: DocumentRef,
  payload: DocumentRef
}
```

The input is:

```text
{
  parameters: <resolved parameter value>,
  premises: [],
  assumptions: [parameters.assumption],
  arguments: DocumentRef
}
```

The rule has `premise_order: "set"`; its input contract admits only an empty
premise set and one assumption ID. Before dispatch, graph admission requires
that ID to equal `parameters.assumption` and exist in the caller-pinned basis.
An unavailable ID is `relation.invalid_assumption`; another schema-valid ID is
`relation.invalid_derivation`. Either error occurs without a semantic call or
fuel. The call is dispatch-ready only when its claim has the profile Claim
contract and its claim, modality, payload, and arguments references resolve.
It then returns:

```text
{
  claim: parameters.claim,
  stance: parameters.stance,
  modality: parameters.modality,
  payload: parameters.payload
}
```

This is a derivation from an explicit caller-approved assumption, not an
observation or truth certificate. Its purpose in the reference environment is
to exercise graph admission, polarity, conflict preservation, dependency
closure, and all four presence states without adding an acquisition system.

## Presence interpreter

Schemas:

- `schemas/interpreter-input.schema.json`
- `schemas/interpreter-output.schema.json`
- `schemas/state.schema.json`

The parameter value is `{}`. The input is:

```text
{
  parameters: {},
  basis: DocumentRef<AssessmentBasis>,
  graph: DocumentRef<EvidenceGraph>,
  targets: [DocumentRef<Claim>]
}
```

The assessor constructs `basis`, `graph`, and `targets` from the admitted basis
and requires canonical equality before dispatch. They are not submitted
semantic results. For each target, in target-reference order, form:

```text
support_basis = sorted IDs of graph nodes whose claim is the target
                and whose stance is support
refute_basis  = sorted IDs of graph nodes whose claim is the target
                and whose stance is refute
```

Exact target-reference equality is used. Node IDs are already unique. Their
sets remain visible as provenance, but cardinality does not affect the
four-way state name or cautious labels. The state name is:

| `support_basis` | `refute_basis` | `state` |
|---|---|---|
| empty | empty | `neither` |
| non-empty | empty | `support_only` |
| empty | non-empty | `refute_only` |
| non-empty | non-empty | `both` |

The output is:

```text
{
  state: {
    revision: "selta.evidence.presence-state/candidate-1",
    targets: [{ claim, support_basis, refute_basis, state }]
  }
}
```

The assessor verifies the result schema, wraps `state` under the profile state
contract, computes its typed-document digest, then inserts it into `D` or reuses
the byte-equal existing entry under T1. It computes the state closures below
before projection. Closures are not interpreter output and do not enter
semantic fuel.

### State dependency closure

There is one closure for each state target entry:

```text
target = { root: "state", component: "/targets/<index>" }
```

Its premises begin with `EvidenceFact` for every matching support or refute
node and then follow the complete admission-justification closure in document
16. In particular, traversal recurses through every derivation premise,
includes the qualified attempt and source grant for every reached atom, and
includes the schedule, selection, and stopping dependencies plus every
applicable recorder and attestation dependency for each reached acquisition.
Non-target claims reached as premises contribute their own claim-theory
dependencies.

Assumptions, grants, and bindings are likewise the full recursive dependency
set. The target claim theory and presence interpreter are always included.
For every state component, `direct_inputs` is exactly the canonical set of
three roots: the target claim reference, basis reference, and graph reference.
Nested parameter, argument, statement, scope, subject, proposition, boundary,
payload, and acquisition references remain recoverable through tagged-reference
traversal and MUST NOT be repeated as direct roots. All collections are
canonical sets.

For an empty graph, premises and transitive assumptions are empty. The claim
theory and interpreter grants and bindings, plus the three direct inputs,
remain present.

State equality is canonical byte equality. Its information preorder is
componentwise set inclusion of `(support_basis, refute_basis)` for equal target
references.

## Cautious projection

Schemas:

- `schemas/projection-parameters.schema.json`
- `schemas/projection-input.schema.json`
- `schemas/projection-output.schema.json`
- `schemas/label.schema.json`

The parameter value is:

```text
{
  affirm: DocumentRef<Label>,
  deny: DocumentRef<Label>
}
```

The binding is dispatch-ready only when both references resolve under the
profile Label contract, carry the respective `affirm` and `deny` values, and
are distinct. Otherwise assessment returns `relation.invalid_binding` at the
projection binding without a semantic call or fuel. The input is:

```text
{
  parameters: <resolved projection parameter value>,
  state: DocumentRef<PresenceState>,
  targets: [DocumentRef<Claim>]
}
```

The assessor constructs the state and target fields and requires the state's
target-reference sequence to equal `targets` before dispatch. Projection
returns:

| State | Canonically ordered labels |
|---|---|
| `support_only` | `[affirm]` |
| `refute_only` | `[deny]` |
| `neither` | `[affirm, deny]` sorted by `JCS(DocumentRef)` |
| `both` | `[affirm, deny]` sorted by `JCS(DocumentRef)` |

The output is:

```text
{
  conclusions: [{ claim, labels }]
}
```

The projection specification maps conclusion index `i` to state component
`/targets/i`. The assessor constructs one closure per conclusion at
`{ root: "conclusion", component: "/conclusions/<index>" }`: it copies the
named state closure, adds the cautious-projection grant and binding, and sets
`direct_inputs` to the canonical union of the copied three roots and the state
document reference. No other root is added. Projection remains a pure function
of its wire input; it never reads or reproduces a hidden prior closure. The
result does not erase the state distinction between `neither` and `both`.

## Resource schedule

All four relations use the single candidate call-cost formula in document 16.
Their domains contain only the finite, basis-bounded collections declared in
their input contracts. This profile makes no implementation-specific
asymptotic claim; the exact dispatch and semantic-fuel schedules in document 16
and the generic basis limits remain authoritative.

## Conformance cases

The profile corpus contains:

- complete closed package/outcome vectors for `neither`, `support_only`,
  `refute_only`, and `both`;
- complete assessment-rejection packages for a duplicate document, wrong graph
  target inventory, unauthorized projection, wrong-role and equal-label
  projection bindings, and a schema-valid wrong assumption;
- forbidden conformance mutations for wrong target, wrong label, empty label
  set, and bad dependency closure; and
- paired `neither`/`both` cases proving that cautious projection may agree while
  the preserved state differs.

Files under `fixtures/negative/` are `assess` inputs paired with exact error
packages; their caller argument is the input package's root reference. Files
under `fixtures/forbidden/` are deliberately wrong implementation outputs, not
inputs to `assess`. A conformance harness must distinguish them from the named
positive oracle value; it must not reinterpret them as epistemic errors emitted
by a self-checking runtime.

No vector is evidence that the profile improves empirical correctness. It
proves only structural compatibility, deterministic semantics, and the laws
advertised here.
