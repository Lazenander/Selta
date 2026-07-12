# 15 — Evidence language and interface

> **Status: S2 normative candidate.** Identifiers ending in `candidate-1` are
> immutable research identifiers, not Selta 0.1 interfaces. A changed candidate
> receives a new identifier. Protocol 1, `verify`, the existing schema grammar,
> and `Report` remain unchanged.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, and **MAY** are
used normatively.

## Minimal surface

The language contains four semantic stages:

```text
acquisition -> evidence -> interpretation -> conclusion projection
```

and one operation:

```text
assess(EvidencePackageSource, AdmittedSemanticEnvironment, ExpectedBasisRef)
  -> EvidencePackage
```

Environment admission is a constructor precondition, specified in document 16;
`assess` is total over raw package source and an admitted environment. The input
package is rooted at an `AssessmentBasis`. A successful output package contains
the input documents, generated state, referenced label documents, and its
outcome document. An error package contains exactly its outcome document.
Every returned package is rooted at an `AssessmentOutcome` and is
document-closed.

`ExpectedBasisRef` is supplied by the caller outside the package. It MUST equal
the package root, including the exact assessment-basis contract and digest.
Targets, graph, assumptions, trust, interpreter, projection, and limits are
therefore caller-approved rather than selected by untrusted package contents.

`assess` does not call a model, start a process, acquire evidence, read a
filesystem, open a socket, repair an answer, retry generation, or choose an
application action.

Probability, voting, reliability, calibration, sequential analysis,
cryptographic audit, argumentation, and K3 are semantic profiles over this
language. None is a core operator.

## Common lexical values

### SafeInt

Every integer is a JSON integer in:

```text
0 .. 9_007_199_254_740_991
```

This is the non-negative IEEE-754 safe-integer range shared by I-JSON and the
candidate schemas. Signed quantities and arbitrary precision values use typed
documents with their own canonical encodings.

### Digest

```text
sha256:<64 lowercase hexadecimal digits>
```

A digest is an integrity identifier, not a confidentiality mechanism.

### ContractId and SemanticsId

Both are `Digest` values with different domain-separated preimages. An
environment maps a `ContractId` to exact schema and evaluation semantics, and a
`SemanticsId` to one normative pure relation. Bare digests used as implementation,
attempt, decision, evidence, assumption, or grant IDs are opaque identities and
are not package reference edges.

## SemanticEnvironment

Schema: `schemas/evidence/candidate-1/environment.schema.json`

The environment is an admitted manifest plus a resolver for the content named
by that manifest. Its digest is bound into every package and outcome. The
resolver supplies the generic language, core semantics, error catalog, schema
sources, and profile semantic specifications; each supplied artifact is checked
against its manifest digest before use. Semantic
implementations are separately accepted by the host as implementations of one
named specification, and their artifact digests are recorded in the outcome.
The environment does not authenticate a native executable. Resolver location
and executable identity are not normative environment input.

```text
SemanticEnvironment {
  revision: "selta.evidence.environment/candidate-1",
  id: Digest,
  language_source: Digest,
  semantics_source: Digest,
  error_catalog_source: Digest,
  evaluation_profiles: [EvaluationProfile],
  contracts: [ContractBinding],
  semantics: [SemanticDescriptor]
}

EvaluationProfile {
  id: Digest,
  raw_value_parser: "strict_duplicate_safe",
  mode: "strict",
  fail_fast: false,
  max_depth: 1,
  max_samples: 1,
  extensions: [{
    name,
    semantic_revision,
    accepted_input: [NodeKind],
    config_schema_source: Digest,
    determinism: "deterministic",
    effect: "pure"
  }]
}

ContractBinding {
  id: ContractId,
  schema_source: Digest,
  schema_language: "selta.schema-language/1",
  evaluation_profile: Digest
}

SemanticDescriptor {
  id: SemanticsId,
  specification_source: Digest,
  kind: "claim_theory | schedule | selection | stopping | attestation
       | extractor | rule | interpreter | projection",
  parameter_contract: ContractId,
  input_contract: ContractId,
  output_contract: ContractId,
  premise_order?: "sequence | set",
  execution_class: "builtin_total"
}
```

The three core sources are the exact UTF-8 bytes of documents 15, 16, and 20
under their distinct `HB` domain tags in document 16. The environment ID binds
them, so changing grammar, operational meaning, or an error rule necessarily
changes every package's environment identity. A profile specification may cite
the core documents without weakening this binding.

Candidate 1 executes only `builtin_total` semantic descriptors whose totality
and abstract cost are part of the reference implementation's conformance
evidence. Declaring an arbitrary callback pure or deterministic is insufficient.
A later metered extension substrate requires a new environment revision.

`output_contract` names one contract. A relation with several result variants
uses one closed Selta union contract; contract-selection order is never
semantic state.

`specification_source` identifies exact UTF-8 Markdown bytes under the domain
tag in document 16. Every invocation input, parameters value, and result is a
typed value admitted by the listed Selta contracts; there is no second
signature language. `premise_order` is present exactly for `rule` descriptors.
Candidate 1 has one language-level cost schedule, also defined in document 16,
so a descriptor cannot substitute its own accounting rule.

Evaluation-profile extensions deliberately do not reference a `SemanticsId`.
Their exact name, semantic revision, accepted-input set, and configuration
schema source bind Selta value verification without creating a hash cycle from
profile to contract to semantic descriptor and back to profile. Evidence
semantic descriptors are the separate relations listed under `semantics`.
`NodeKind` is one of `null`, `bool`, `int`, `float`, `str`, `object`, or
`array`. A Selta declaration accepting any node kind lists all seven.

The environment contains normative semantics, not native executable identity.
The implementation that actually ran is recorded in the outcome. Thus document
and claim identities remain stable across conforming implementations and
platforms.

## DocumentRef

A reference is an explicitly tagged, closed object:

```json
{
  "$ref": "sha256:...",
  "contract": "sha256:..."
}
```

`$ref` identifies a `TypedDocument`; `contract` states its expected
`ContractId`. This shape is reserved by the evidence package language and is
the only generic document edge. All other bare digests are non-reference
identities.

Package closure follows tagged references recursively, including references in
application-defined typed values. A literal application value requiring this
exact reserved shape must be sealed or encoded under another representation.

## TypedDocument

```text
TypedDocument {
  contract: ContractId,
  digest: Digest,
  value: JSON
}
```

The environment MUST contain the contract. Package admission verifies the value
under its exact evaluation profile and recomputes its document digest.

Private content MAY use an application-defined sealed or blinded value under a
contract that states its disclosure and verification rules. There is no second
ambient private-value format.

## SemanticBinding

```text
SemanticBinding {
  semantics: SemanticsId,
  parameters: DocumentRef
}
```

The environment descriptor states the parameter contract and exact relation.
Executable identity is deliberately absent. Every application of a binding is
validated under the semantic judgment in document 16; naming a trusted
semantics does not authenticate a submitted output.

## EvidencePackage

Schema: `schemas/evidence/candidate-1/package.schema.json`

```text
EvidencePackage {
  language: "selta.evidence.package/candidate-1",
  environment: Digest,
  documents: [TypedDocument],
  root: DocumentRef
}
```

The environment field MUST equal the supplied `SemanticEnvironment.id`.
Documents are sorted by digest and unique. Closure from `root` MUST include
every document exactly once; missing and unreachable documents are errors. The
environment, schema sources, and executable artifacts are explicit trust
anchors external to document closure, not hidden package documents.

An input root MUST have the `AssessmentBasis` contract. A successful output
root MUST have the `AssessmentOutcome` contract.

## Claim

Schema: `schemas/evidence/candidate-1/claim.schema.json`

```text
Claim {
  revision: "selta.evidence.claim/candidate-1",
  theory: SemanticBinding,
  subject: DocumentRef,
  proposition: DocumentRef,
  boundary: DocumentRef
}
```

The theory canonicalizes the three referenced values before the claim document
is admitted. Consequently claim document digest is claim identity: equal claims
under one theory MUST have equal canonical documents. Assessor identity does
not enter the claim unless it is part of the proposition.

Incompatibility and optional negation are internal to one claim theory.
Different theories are unrelated unless a typed derivation rule explicitly
bridges them.

## Acquisition

Schema: `schemas/evidence/candidate-1/acquisition.schema.json`

```text
Acquisition {
  revision: "selta.evidence.acquisition/candidate-1",
  manifest: Manifest,
  trace: Trace
}

Manifest {
  frame: DocumentRef,
  channel: DocumentRef,
  context: DocumentRef,
  sources: [DocumentRef],
  schedule: SemanticBinding,
  selection: SemanticBinding,
  stopping: SemanticBinding,
  budget: DocumentRef,
  randomness?: DocumentRef
}

Trace {
  decisions: [Decision],
  attempts: [Attempt],
  stop: Digest,
  accounting: AccountingAttestation
}
```

The manifest is a declaration. A valid trace separately proves, by semantic
validation, that each recorded decision matches its bound relation.

### Decision

```text
Decision {
  id: Digest,
  manifest: Digest,
  kind: "schedule | select | stop",
  policy: SemanticBinding,
  observed_plans: [Digest],
  input: DocumentRef,
  output: DocumentRef
}
```

Decisions are content-addressed events. `manifest` is the exact acquisition
manifest digest. `observed_plans` declares a downward-closed causal prefix of
attempt plan IDs. `input` is the typed policy-input snapshot submitted for
validation. These fields bind declared provenance; they do not prove that an
external policy process lacked unrecorded or future information. A later
non-anticipation profile may use a core-constructed snapshot. The policy output
MUST validate against the scheduled attempt proposal, disposition proposal, or
stop transition that references the decision.

A schedule output contains the proposed source, request, context view,
ordinal, and predecessor plan IDs, but not the schedule decision ID or plan
ID. The decision ID is therefore computed first; the attempt plan ID then
binds that decision. A selection output contains the disposition kind and
reason but not its decision ID. This order prevents cyclic identities.

### Attempt

```text
Attempt {
  plan_id: Digest,
  id: Digest,
  ordinal: SafeInt,
  predecessors: [Digest],
  scheduled_by: Digest,
  source: DocumentRef,
  request: DocumentRef,
  context_view: DocumentRef,
  outcome: AttemptOutcome,
  disposition: Disposition
}

Disposition {
  kind: "include | exclude | censor",
  decided_by: Digest,
  reason: DocumentRef
}
```

`plan_id` is fixed before execution. Completed `id` additionally binds the
outcome and disposition, preventing two packages from assigning different
results to one attempt identity. `scheduled_by` references a `schedule`
decision; `decided_by` references a `select` decision.

### AttemptOutcome

```text
Observed    { kind: "observed", response: DocumentRef }
Malformed   { kind: "malformed", raw: DocumentRef, error: DocumentRef }
Unavailable { kind: "unavailable", reason: DocumentRef }
```

Malformed wire bytes are carried in a typed byte or sealed document; invalid
JSON bytes are never inserted directly into the package JSON.

Legal combinations are:

| Outcome | Dispositions |
|---|---|
| `observed` | `include`, `exclude`, `censor` |
| `malformed` | `exclude`, `censor` |
| `unavailable` | `exclude` |

Only `observed + include` may directly feed an evidence atom.

### AccountingAttestation

```text
Unattested { kind: "unattested" }

Scoped {
  kind: "scoped",
  recorder: DocumentRef,
  scope: DocumentRef,
  checkpoint: DocumentRef,
  exceptions: [DocumentRef],
  attestation: DocumentRef,
  verifier: SemanticBinding
}
```

A scoped attestation attributes one mechanically bounded statement. Its
`attestation` document is the submitted assertion, not a stored verifier
result. The verifier authenticates or otherwise validates that assertion and
returns the accepted or rejected result fixed by its bound specification. A
scoped record is valid only for an accepted result; no verifier-result document
is inserted into the package. Acceptance does not by itself prove that the
trace is complete, nor establish world completeness, semantic recall, truth,
or correct wall time.

Zero-attempt traces are legal when the validated stop decision declares the
empty prefix. Otherwise the stop decision MUST declare the complete recorded
attempt set.

## EvidenceGraph

Schema: `schemas/evidence/candidate-1/graph.schema.json`

```text
EvidenceGraph {
  revision: "selta.evidence.graph/candidate-1",
  acquisitions: [DocumentRef],
  claims: [DocumentRef],
  nodes: [EvidenceNode]
}
```

Multiple acquisitions compose without inventing one artificial trace.

```text
AttemptRef {
  acquisition: DocumentRef,
  attempt: Digest
}
```

### Atom

```text
Atom {
  kind: "atom",
  id: Digest,
  claim: DocumentRef,
  stance: "support | refute",
  attempt: AttemptRef,
  extractor: SemanticBinding,
  modality: DocumentRef,
  payload: DocumentRef
}
```

The extractor relation is re-evaluated. Its canonical output MUST equal the
claim, stance, modality, and payload in the submitted atom.

### Derivation

```text
Derivation {
  kind: "derive",
  id: Digest,
  claim: DocumentRef,
  stance: "support | refute",
  rule: SemanticBinding,
  premises: [FactRef],
  assumptions: [Digest],
  modality: DocumentRef,
  payload: DocumentRef,
  arguments: DocumentRef
}
```

The rule relation is re-evaluated. Its output MUST equal the submitted claim,
stance, modality, and payload.

### FactRef

Exactly one tagged variant is present:

```text
EvidenceFact     { evidence: Digest }
AttemptFact      { attempt: AttemptRef }
TraceFact        { trace: DocumentRef }
AccountingFact   { accounting: DocumentRef }
```

`TraceFact` and `AccountingFact` reference an acquisition document under the
corresponding field name. A missingness rule can therefore inspect a trace and
scoped accounting attestation without treating absence as an atom. Its query
and selection assumptions remain explicit derivation arguments and
assumptions.

Nodes are stored by lexical ID. Storage order has no topological meaning;
acyclicity is checked independently over evidence-premise edges.

## AssessmentBasis

Schema: `schemas/evidence/candidate-1/basis.schema.json`

```text
AssessmentBasis {
  revision: "selta.evidence.basis/candidate-1",
  targets: [DocumentRef],
  graph: DocumentRef,
  trust: TrustBasis,
  assumptions: [Assumption],
  interpreter: SemanticBinding,
  projection: SemanticBinding,
  limits: ResourceLimits
}
```

The graph identifies every acquisition; the basis does not repeat them.

### TrustBasis

Candidate 1 has three exact, non-delegating grants:

```text
TrustBasis {
  id: Digest,
  grants: [Grant]
}

SourceGrant {
  role: "source",
  id: Digest,
  source: DocumentRef,
  scope: DocumentRef
}

RecorderGrant {
  role: "recorder",
  id: Digest,
  recorder: DocumentRef,
  scope: DocumentRef
}

SemanticGrant {
  role: "semantic",
  id: Digest,
  semantics: SemanticsId
}
```

Scope matching is exact `DocumentRef` equality. There is no containment,
delegation, expiry, supersession, or revocation language in candidate 1.
Historical and current trust changes are represented by different immutable
bases. The trust ID uses the domain-separated identity in document 16 and is
itself bound into the caller-pinned assessment basis.

Authorization does not imply truth, competence, independence, calibration, or
completeness.

### Assumption

```text
Assumption {
  id: Digest,
  statement: DocumentRef,
  scope: DocumentRef
}
```

`scope` is part of the assumption's content identity, not an ambient hierarchy
or containment rule. A rule consumes the exact assumption ID and therefore the
exact statement/scope pair. Any relationship between scopes must be stated by
that rule or another explicit derivation.

### ResourceLimits

Every field is a `SafeInt`:

```text
max_documents
max_total_document_bytes
max_attempts_total
max_graph_nodes
max_graph_edges
max_fan_in
max_graph_depth
max_semantic_calls
max_semantic_fuel
max_state_bytes
max_conclusion_labels_total
```

The exact cost model is in document 16. Zero is meaningful: it forbids any
charged quantity in that category and may therefore make a concluded outcome
impossible without making the basis malformed.

## AssessmentOutcome

Schema: `schemas/evidence/candidate-1/outcome.schema.json`

A concluded outcome document is:

```text
Concluded {
  revision: "selta.evidence.outcome/candidate-1",
  status: "concluded",
  basis: DocumentRef,
  environment: Digest,
  state: DocumentRef,
  closures: [DependencyClosure],
  conclusions: [{ claim: DocumentRef, labels: [DocumentRef] }],
  executions: [{ semantics: SemanticsId, implementation: Digest }],
  resources: ResourceUsage
}

DependencyClosure {
  target: { root: "state | conclusion", component: JSONPointer },
  premises: [FactRef],
  assumptions: [Digest],
  grants: [Digest],
  bindings: [SemanticBinding],
  direct_inputs: [DocumentRef]
}
```

State closures exclude projection bindings. Conclusion closures include the
projection binding and the state document or other direct inputs they consume;
the assessor derives both closure kinds from the fixed dependency functions in
the semantic specifications rather than asking semantic outputs to reproduce
hidden prior closures.

`direct_inputs` is the exact canonical set of root `DocumentRef` values named
by that dependency function. It is not a flattened reference closure. Every
tagged reference transitively reachable from a root remains recoverable, but
MUST NOT also appear in `direct_inputs` unless the specification independently
names it as a root. Parameters remain recoverable through the bindings that
name them.

Candidate 1 closures are full admission-justification closures, not runtime
read traces: the union of their premises, assumptions, grants, bindings,
direct-input roots, and transitively tagged references recovers every evidence,
acquisition, claim, parameter, and document needed to admit and justify the
component under the fixed builtin dependency function in document 16.

Every target has a non-empty label set. A singleton is resolved; a larger set
is unresolved. The output package contains the generated state document and
the referenced input label documents and remains closed.

An error outcome document is:

```text
ErrorOutcome {
  revision: "selta.evidence.outcome/candidate-1",
  status: "error",
  environment: Digest,
  errors: [{ phase, code, path, message }],
  executions: [{ semantics: SemanticsId, implementation: Digest }],
  resources: ResourceUsage
}
```

Errors are not conclusion labels and never become evidence. Candidate 1 has no
open-ended error-detail payload; stable code, phase, pointer, and message are
the complete error record. The caller already supplied `ExpectedBasisRef`, so
the error outcome does not repeat or retain an untrusted basis reference. Its
package contains exactly the outcome document and remains document-closed.

`ResourceUsage` has the same counters in concluded and error outcomes:

```text
ResourceUsage {
  documents: SafeInt,
  total_document_bytes: SafeInt,
  attempts_total: SafeInt,
  graph_nodes: SafeInt,
  graph_edges: SafeInt,
  max_fan_in_observed: SafeInt,
  graph_depth: SafeInt,
  semantic_calls: SafeInt,
  semantic_fuel: SafeInt,
  state_bytes: SafeInt,
  conclusion_labels_total: SafeInt
}
```

## Canonical collection order

| Collection | Meaning and order |
|---|---|
| Environment profiles, contracts, semantics | Sets by ID |
| Evaluation-profile extensions | Set by `(name, semantic_revision)` |
| Extension accepted-input kinds | Set in `null, bool, int, float, str, object, array` rank order |
| Package documents | Set by document digest |
| Manifest sources | Set by `JCS(DocumentRef)` |
| Decisions | Set by decision ID |
| Attempts | Sequence by `(ordinal, completed ID)` |
| Predecessors and observed plans | Sets by attempt plan ID |
| Accounting-attestation exceptions | Set by `JCS(DocumentRef)` |
| Acquisitions, claims, targets | Sets by `JCS(DocumentRef)` |
| Evidence nodes | Set by node ID |
| Ordered rule premises | Semantic sequence |
| Unordered rule premises | Set by `JCS(FactRef)` |
| Grants and assumptions | Sets by ID |
| Closures | Set by `(target.root, target.component)` |
| Premises, assumption IDs, grant IDs, bindings, and direct inputs in a closure | Canonical sets |
| Conclusions | Set by claim reference |
| Labels | Set by document reference |
| Execution records | Set by semantics ID |
| Errors | Sequence by `(phase, path, code)` |

Every set is duplicate-free. Unless a row states a more specific key, set
members are ordered by ascending UTF-8 bytes of `JCS(member)`. A noncanonical
input is rejected, not repaired.

The environment has at least one evaluation profile, contract, and semantic
descriptor. Each descriptor has exactly one output contract. Structurally
optional collections may be empty, but relational admission remains
authoritative: graph claims exactly inventory the non-empty targets and node
claims, and a concluded outcome has non-empty closures and execution records.
Decisions are non-empty because even an empty trace has one stop decision.
Assessment targets, concluded conclusions, every concluded label set, and
error-outcome errors are non-empty. Evaluation-profile `max_depth` is positive
and at most `u32::MAX`;
candidate 1 fixes `max_samples` to one because all admitted schema verifiers are
deterministic.

## Interface phases

```text
1. require an already admitted SemanticEnvironment
2. duplicate-safe parse the package source
3. structurally verify the wrapper and typed document values
4. validate identities and close typed references
5. match the caller-supplied ExpectedBasisRef
6. validate static relations and every submitted semantic transition
7. interpret
8. project
9. return a closed outcome package
```

The exact substage, suppression, ordering, pointer, charged-work, and truncation
rules are fixed by document 20. Candidate error codes and English messages are
protocol values, not implementation prose.

## Deliberate exclusions

Candidate 1 adds no:

- general permission or delegation system;
- trust lifecycle language;
- default probability, confidence, vote, or source reliability;
- provider, prompt, model, retry, or repair concept;
- application action or workflow decision;
- persistence, queue, HTTP, or daemon contract;
- native path, process, signal, socket, clock, or platform type; or
- modification to Selta 0.1 schemas, reports, hosts, or protocol 1.

The result is a small evidence-and-provenance calculus, not a universal
uncertainty framework.
