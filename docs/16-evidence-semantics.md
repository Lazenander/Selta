# 16 — Evidence semantics and laws

> **Status: S2 normative candidate.** This document defines the meaning of
> [15-evidence-language.md](15-evidence-language.md). It is intentionally a
> small, replayable calculus. An implementation is conforming only when a user
> can predict its accepted values and semantic outputs from these documents,
> the candidate Selta schemas, and the bound semantic specifications without
> reading implementation source.

## Scope and notation

`assess` accepts raw package bytes and an already admitted semantic
environment:

```text
assess(package_source, Sigma, expected_basis_ref) -> closed outcome package
```

Environment admission is a constructor precondition because an invalid
environment cannot supply a trustworthy contract for its own error output.
The environment constructor is nevertheless specified below and every
serialized environment value has a Selta schema.

The normative judgments are:

```text
admit_environment(environment_source, resolver) => Sigma
Sigma |- admit(package_source) => D, basis
Sigma, D, basis |- relations valid
Sigma, D, basis |-I graph => state, closures
Sigma, D, basis, state |-P conclusions, closures
```

`D` is a typed, document-closed content map. Each judgment is deterministic.
Malformed `package_source` under an admitted `Sigma` produces an error outcome,
never an epistemic conclusion. Environment rejection occurs before `assess` and
has the separate boundary stated below.

## Semantic environment

An admitted environment is:

```text
Sigma = admitted manifest + verified artifacts + conforming implementations
```

The manifest is the value in
`schemas/evidence/candidate-1/environment.schema.json`. The resolver supplies
the UTF-8 Markdown language, core-semantics, error-catalog, and profile semantic
specifications plus the schema sources named by digest.
A separate host implementation registry supplies one implementation and its
artifact digest for each semantic specification. The constructor:

1. performs duplicate-safe admission of the manifest;
2. verifies its candidate Selta schema under the bootstrap pure-builtin
   profile;
3. recomputes every embedded identity and the environment ID;
4. requires every set to be unique and canonically ordered;
5. fetches every named schema or specification artifact and checks its digest
   before use, including the exact source bytes of documents 15, 16, and 20;
6. admits every schema source through the exact evaluation profile named by
   its contract, checking each profile extension's name, semantic revision,
   accepted-input set, configuration schema source, determinism, and effect;
7. closes every internal manifest reference: each contract's
   `evaluation_profile` resolves to exactly one admitted profile; each profile
   extension's `config_schema_source` and each contract's `schema_source`
   resolves to digest-verified admitted schema content; and each semantic
   descriptor's `specification_source` resolves to exact digest-verified bytes,
   while its `parameter_contract`, `input_contract`, and `output_contract` each
   resolve to exactly one admitted contract;
8. resolves exactly one contract for each candidate core role in the table
   below; and
9. binds exactly one host-accepted implementation and recorded artifact digest
   to each semantics ID for the lifetime of `Sigma`.

Missing, colliding, or ambiguous internal references reject construction; the
constructor never retains a partially resolved environment.

Core roles are derived from exact schema-source identity; they are not another
manifest field or a name trusted from the caller. For each row, the constructor
admits the named repository schema, computes its canonical admitted-`Node`
identity, and requires exactly one contract whose `schema_source` has that
identity, whose `schema_language` is `selta.schema-language/1`, and whose
`evaluation_profile` is the exact candidate profile. Zero or multiple matches
reject the environment.

| Core role | Exact candidate schema |
|---|---|
| environment manifest | `schemas/evidence/candidate-1/environment.schema.json` |
| package wrapper | `schemas/evidence/candidate-1/package.schema.json` |
| acquisition | `schemas/evidence/candidate-1/acquisition.schema.json` |
| claim | `schemas/evidence/candidate-1/claim.schema.json` |
| graph | `schemas/evidence/candidate-1/graph.schema.json` |
| assessment basis | `schemas/evidence/candidate-1/basis.schema.json` |
| assessment outcome | `schemas/evidence/candidate-1/outcome.schema.json` |

The resulting role map, rather than a package-selected contract, supplies S0's
wrapper contract, the required input-root contracts, and the sole outcome
contract used for both success and error packages. Additional contracts remain
ordinary typed-value contracts and cannot replace a core role.

Environment construction is an accept/reject trust-bootstrap boundary, not an
assessment. Rejection is reported out of band by the host constructor: it has
no candidate error code, pointer, resource counters, or serialized outcome
package. A future stable diagnostic format would be a separate bootstrap
protocol and cannot be selected by the environment being rejected.

Candidate 1 accepts only `builtin_total` semantic descriptors. Such a
descriptor names a finite, deterministic relation whose totality, dependency
behavior, and cost schedule are part of its conformance corpus. Merely
declaring a callback pure, deterministic, or bounded is insufficient. Native,
Wasm, process, model, and network callbacks are outside candidate 1.

Implementation acceptance is an external host conformance precondition, not a
self-authenticating field in the environment. Environment admission verifies
schema and specification artifacts; it records but cannot infer the correctness
of the separately supplied implementation artifact. It does not consume an
undefined “conformance record.”

Registration provides capability, not authority. Every semantic binding used
by an assessment also needs an exact `semantic` grant in the assessment basis,
whose exact root reference must equal `expected_basis_ref` supplied outside the
package.

The identity dependency graph is acyclic: raw language, semantics, error, and
profile-specification source IDs are leaves; schema-source IDs precede
evaluation profiles; profile IDs precede contracts; contract IDs precede
evidence semantic descriptors; all of those precede the environment ID.
Evaluation-profile extensions therefore bind stable Selta builtin revisions
directly and do not reference evidence semantic descriptors.

## Three distinct admission stages

The term *admission* MUST NOT conflate these stages:

1. **Schema-source admission.** The candidate schema files are admitted by
   `Registry::with_pure_builtins().admit_source(source, policy)` under the exact
   profile below.
2. **Raw package parsing.** A future evidence lane parses UTF-8 JSON while
   detecting duplicate keys before projection to a generic JSON value.
   It performs no fence stripping, repair, trailing-comma recovery, or numeric
   coercion.
3. **Typed-value verification.** Each parsed value is verified in strict mode
   by its already admitted Selta schema and exact evaluation profile.

Current Selta schema-source admission implements stage 1. Current arbitrary
value intake does not implement stage 2 and this candidate does not claim that
it does. Stage 2 should extract one neutral, bounded unique-JSON parser reusable
by strict schema admission and the later pure evidence crate; it MUST NOT add a
second schema evaluator or change V1 `MetaIssue` behavior.

The raw parser accepts only I-JSON values and rejects duplicate names at every
object depth. Invalid UTF-8, invalid syntax, or a duplicate name returns one
parse-phase error at the earliest offending byte and stops. Later phases use
the collect-and-sort rule. This first-error boundary is deliberately small and
bounds error amplification.

### Exact Selta evaluation profile

The profile's extension set MUST equal the name-sorted declaration projection
from `Registry::with_pure_builtins()`: `len`, `non_empty`, `one_of`, `range`,
and `regex`, each with its exact `selta.builtin.<name>.v1` revision, seven-kind
accepted-input projection, configuration schema source, deterministic
declaration, and pure effect. A subset is not accepted because stable Selta has
no filtered-registry constructor.

Schema sources contain no `$env` reference. Registry admission uses exactly:

```text
AdmissionPolicy::pure_only().with_builtin_limits(
    BuiltinAdmissionLimits::new(Some(128), Some(512))
)
```

Before registry admission, one schema source is limited to 1,048,576 UTF-8
source bytes, nesting depth 128, and 100,000 JSON values. At every `Node.len`
position recognized by the closed Selta grammar, each present `min` or `max`
MUST be at most 4,294,967,295. This neutral-tree check occurs before stable
Selta converts a bound to `usize`; host pointer width therefore cannot change
candidate admission.

All candidate parser ceilings use one counting convention. The root is one
value at depth one. Every scalar, object, and array is one value; each object
member value and array element is counted recursively, while object member
names are not values. A child value has parent depth plus one, including a
scalar child. String limits measure UTF-8 bytes after JSON escape decoding and
apply to both member names and string values. A ceiling is inclusive.

After the preflight, the stable public API sequence for value verification is
exactly equivalent to:

```rust
let registry = Registry::with_pure_builtins();
let policy = AdmissionPolicy::pure_only().with_builtin_limits(
    BuiltinAdmissionLimits::new(Some(128), Some(512)),
);
let admitted = registry.admit_source(source, &policy)?;
let cache = NoCache;
let runtime = Runtime::new(&registry, &cache);
let options = Options {
    mode: Mode::Strict,
    fail_fast: false,
    max_depth: 1,
    max_samples: 1,
    deadline_ms: None,
    schema_name: None,
};
let env = serde_json::json!({});
let report = verify(
    admitted.as_node(),
    Input::Value(value),
    &env,
    &options,
    &runtime,
).await;
let success = report.verdict == Verdict::Pass && report.errors.is_empty();
```

`Input::Text` is never used. `raw_value_parser` belongs to the assessor, not to
Selta `Options`. `max_samples` is not treated as a deterministic-call limit;
the fixed schema source and outer input ceilings bound structural work.

## Canonical encoding and identities

Candidate 1 uses RFC 8785 JSON Canonicalization Scheme (`JCS`) over I-JSON.
Every integer is a `SafeInt` from document 15. Unicode is not normalized.

For JSON values define:

```text
H(tag, value) = SHA-256(UTF8(tag) || 0x00 || JCS(value))
```

For raw artifact bytes define:

```text
HB(tag, bytes) = SHA-256(UTF8(tag) || 0x00 || bytes)
```

Both are displayed as `sha256:<lowercase hexadecimal>`. The following table is
the complete identity preimage schedule. `without id` means removal of exactly
the top-level `id` member and no other normalization.

| Identity | Domain tag | Preimage value or bytes |
|---|---|---|
| Schema source | `selta.evidence.schema-source/candidate-1` | canonical admitted `Node` projection defined below |
| Language specification | `selta.evidence.language-specification/candidate-1` | exact document 15 bytes via `HB` |
| Core semantics | `selta.evidence.core-semantics/candidate-1` | exact document 16 bytes via `HB` |
| Error catalog | `selta.evidence.error-catalog/candidate-1` | exact document 20 bytes via `HB` |
| Evaluation profile | `selta.evidence.evaluation-profile/candidate-1` | profile without `id` |
| Contract | `selta.evidence.contract/candidate-1` | contract binding without `id` |
| Semantic specification | `selta.evidence.semantic-specification/candidate-1` | exact specification bytes via `HB` |
| Semantic descriptor | `selta.evidence.semantics/candidate-1` | descriptor without `id` |
| Environment | `selta.evidence.environment/candidate-1` | manifest without `id` |
| Typed document | `selta.evidence.document/candidate-1` | `{ "contract": contract, "value": value }` |
| Acquisition manifest | `selta.evidence.manifest/candidate-1` | exact `Manifest` value |
| Decision | `selta.evidence.decision/candidate-1` | decision without `id` |
| Attempt plan | `selta.evidence.attempt-plan/candidate-1` | object listed below |
| Completed attempt | `selta.evidence.attempt/candidate-1` | `{ "plan_id": plan_id, "outcome": outcome, "disposition": disposition }` |
| Atom | `selta.evidence.atom/candidate-1` | atom without `id` |
| Derivation | `selta.evidence.derivation/candidate-1` | derivation without `id` |
| Assumption | `selta.evidence.assumption/candidate-1` | assumption without `id` |
| Grant | `selta.evidence.grant/candidate-1` | grant without `id` |
| Trust basis | `selta.evidence.trust/candidate-1` | trust basis without `id` |
| Implementation artifact | `selta.evidence.implementation/candidate-1` | exact artifact bytes via `HB` |

The attempt-plan preimage is exactly:

```json
{
  "manifest": "<manifest digest>",
  "ordinal": 0,
  "predecessors": [],
  "scheduled_by": "sha256:...",
  "source": { "$ref": "sha256:...", "contract": "sha256:..." },
  "request": { "$ref": "sha256:...", "contract": "sha256:..." },
  "context_view": { "$ref": "sha256:...", "contract": "sha256:..." }
}
```

The literal `ordinal` above is illustrative; the actual SafeInt is used.
Collections inside every preimage MUST already have the canonical order in
document 15. Reordering a set is rejected rather than normalized before
hashing.

A digest is a computational content identity under SHA-256 collision
resistance, not a logical proof that unavailable bytes are equal. Whenever a
judgment would identify two simultaneously available values under one identity
domain and collection or content map, it MUST compare their canonical bytes.
Equal bytes reuse one identity entry. Unequal bytes produce
`identity.collision`, and the judgment stops before coalescing or inserting the
later value.

### Canonical admitted `Node` projection

Schema-source identity binds the admitted schema meaning, not whitespace or
default spelling. After strict source admission, project a `Node` to JSON as
follows before JCS:

- every node contains `type` and all type-specific members;
- an object always contains `fields` (possibly empty) and explicit `open`;
- every object field contains explicit `required`;
- an array contains `item` and contains `len` only when a bound was supplied;
  absent `min` or `max` remains absent;
- a union contains its ordered `variants`;
- `verify` is absent when empty and otherwise preserves verifier order;
- `description` is absent when unspecified;
- a verifier leaf always contains `ext` and `config` (default `{}`), and
  contains `sampling` only when supplied;
- a supplied sampling block contains explicit `samples`, `vote`, and `depth`
  after Selta defaults, plus `min_valid` only when supplied; and
- combinator discriminants and `not.message` retain their admitted values.

This is the frozen candidate projection of Selta 0.1 `Node`, `Field`,
`VerifierSpec`, and `Sampling`. A resolver compares the projection of the
runtime declaration schema with the manifest digest; it never depends on an
unavailable builtin raw source.

## Package admission

Before a basis is available, the candidate conformance domain has fixed outer
ceilings:

```text
raw package bytes        <= 67_108_864
raw JSON nesting depth   <= 128
raw JSON values          <= 1_000_000
one string's UTF-8 bytes <= 16_777_216
```

Exceeding one produces `resource.outer_exceeded`. These ceilings protect the
operation that reads the inner limits; they are not application policy.
Depth, value, and decoded-string bytes use the exact common counting convention
under the evaluation profile above. Raw package bytes are the length of the
supplied byte slice before UTF-8 decoding.

After raw parsing, admission proceeds in this order:

1. verify the package wrapper under its candidate schema;
2. require its environment digest to equal `Sigma.id`;
3. reject duplicate or noncanonically ordered documents;
4. resolve each contract and verify each value under its exact profile;
5. recompute typed-document and embedded identities;
6. traverse only tagged `DocumentRef` objects and verify target contracts;
7. reject missing references, cycles in document references, and unreachable
   documents;
8. require the input root contract to be `AssessmentBasis` and exact equality
   with the caller's `expected_basis_ref`;
9. recompute the trust-basis ID;
10. enforce the admitted basis limits over charged input and generated work.

Bare digests are opaque identities and never closure edges. Tagged references
inside application-defined values are closure edges. A successful output
contains the complete admitted input closure and generated documents. An error
output contains exactly its outcome document.

## Semantic application

Every semantic call uses the judgment:

```text
Sigma, D |- apply(binding, input) => output, fuel
```

It performs all of the following:

1. resolve `binding.semantics`, verify `binding.parameters` against the
   descriptor's parameter contract, and require the ephemeral input
   `parameters` member to equal the resolved parameter value canonically;
2. require the descriptor kind and exact input contract expected at the call
   site;
3. require an exact semantic grant for the semantics ID;
4. verify the ephemeral input value under the descriptor's input contract and
   every call-domain predicate fixed by the semantic specification;
5. execute the bound total relation, resolving only document references that
   the semantic specification declares readable from `D`;
6. record the actual implementation and charge the exact call and fuel cost;
7. verify the output under the descriptor's single output contract; and
8. compare `JCS(output)` with the submitted output when validating an existing
   decision, atom, derivation, or attestation.

Preparation through step 4 is total over admitted values and does not dispatch
or charge fuel. A parameter-value predicate failure is
`relation.invalid_binding`. An input-contract or call-domain failure for a
claim-theory canonicalization is also `relation.invalid_binding`, anchored at
that claim's theory binding. The corresponding failure for an existing
decision, atom, derivation, or attestation uses that record's specific
`relation.invalid_*` code. Interpreter and projection inputs are constructed by
the assessor from already admitted values; their exact construction is part of
the bound specifications. A conforming assessor therefore cannot create an
out-of-domain generated call.

`builtin_total` means total on this explicitly admitted call domain, not that a
native function may trap or invent behavior for a rejected input. Every
call-domain predicate and its error mapping is normative specification data;
there is no implementation-private precondition.

Naming a trusted relation never authenticates a submitted result. Re-execution
and canonical comparison are mandatory.

For all candidate reference semantics, one call costs:

```text
1 + UTF8_bytes(JCS(invocation)) + UTF8_bytes(JCS(output))
```

where `invocation` is exactly `{ "binding": binding, "input": input }` with
resolved parameter value included in `input`. SafeInt overflow is an error.

The call sites are:

| Kind | Exact logical input | Submitted or generated output checked |
|---|---|---|
| `claim_theory` | operation tag plus subject, proposition, boundary, and parameters | canonical claim triple, incompatibility result, or optional negation result |
| `schedule` | decision `input`, manifest digest, and `observed_plans` | attempt proposal referenced by decision `output` |
| `selection` | decision `input`, manifest digest, and `observed_plans` | disposition proposal referenced by decision `output` |
| `stopping` | decision `input`, manifest digest, and `observed_plans` | stop transition referenced by decision `output` |
| `attestation` | recorder, scope, checkpoint, exceptions, attestation, and parameters | exactly an accepted or rejected attestation result |
| `extractor` | qualified completed attempt, resolved observed response, and parameters | claim, stance, modality, and payload in the atom |
| `rule` | ordered resolved premises, listed assumptions, arguments, and parameters | claim, stance, modality, and payload in the derivation |
| `interpreter` | admitted basis and graph | state |
| `projection` | state, targets, and parameters | conclusion labels |

The contracts named by each descriptor make these logical records concrete.
Each candidate descriptor has one scalar `output_contract`; alternatives are
variants of one closed union schema. Different shapes require different
descriptors and therefore different IDs.

### Dispatch schedule

One assessment builds and executes exactly this schedule; referring to one
record more than once never schedules it again:

1. one claim-theory canonicalization for each claim in `graph.claims`, in
   canonical claim-reference order;
2. for each acquisition in `graph.acquisitions`, in canonical reference order,
   one policy call for each decision in decision-ID order, followed by one
   attestation call when its accounting variant is `scoped`;
3. one extractor or rule call for each evidence node, in node-ID order;
4. one interpreter call after the relation phase has no errors; and
5. one projection call after interpretation has no errors.

R2 executes entries strictly in this order and stops after the first failed
call; it never guesses whether a later call is independent of the failure. An
entry whose preparation fails was already rejected in R1 and charges no call
or fuel. Dependency-closure construction, repeated references, and memoization
never dispatch semantics. Results are reused only within this one assessment;
no cache result or prior assessment changes the counters.

For each scheduled entry, precedence is exact:

1. if adding one would exceed `max_semantic_calls`, return
   `resource.basis_exceeded` without dispatch;
2. dispatch, increment `semantic_calls`, and record its implementation;
3. compute the call fuel and checked-add it to the prior total; arithmetic
   overflow returns `evaluation.arithmetic_overflow` and leaves
   `semantic_fuel` at the prior representable total;
4. otherwise commit the new fuel total, even when it exceeds the configured
   limit; if it does, return `evaluation.fuel_exhausted`;
5. validate the sole output contract, returning `evaluation.invalid_output` on
   failure; then
6. for a submitted transition, compare canonical output and return
   `relation.claim_noncanonical` for a claim-theory inequality or
   `relation.semantic_mismatch` for every other inequality.

Any error stops the schedule. Later phases do not run after an earlier-phase
error. This order makes fuel exhaustion dominate an output mismatch on the
same call and arithmetic overflow dominate both.

## Claim semantics

A claim is admitted only after its theory's `canonicalize` operation returns
the exact submitted subject, proposition, and boundary references. Thus the
typed claim-document digest is canonical claim identity under that theory.
Two noncanonical representations of one theory-level claim cannot both be
admitted.

The canonicalizer MUST be deterministic, contract-correct, and idempotent.
Theory equality is defined as byte equality of canonical triples. A theory MAY
define incompatibility or partial negation as additional operations; their
algebraic laws belong to that named specification and are not candidate-core
operators. Claims under different theories are unrelated. A typed derivation
rule is required to bridge them. Refutation never becomes support for a
negated claim without such a rule.

## Acquisition semantics

The manifest's `sources` array is the exact eligible source set for candidate
1. `frame`, `channel`, `context`, and `budget` remain typed declarations passed
to policies; the core does not invent a frame-membership, context-projection,
or budget algebra. A valid attempt source is an exact member of `sources`.
Only an atom that directly consumes an `observed + include` attempt requires a
source grant, and that grant's source and scope must equal the attempt source
and manifest `frame`. Every legal outcome/disposition combination not consumed
by an atom—`observed + exclude`, `observed + censor`, `malformed + exclude`,
`malformed + censor`, and `unavailable + exclude`—requires no source grant.
`AttemptFact`, `TraceFact`, and `AccountingFact` likewise do not consume source
content; their policy, recorder, and attestation dependencies remain exactly
those specified below.

Operational fact resolution exposes acquisition structure and the identities
of its nested `DocumentRef` values, but does not follow those references. In
particular, the resolver supplied to a rule MUST NOT dereference an
`Observed.response` or `Malformed.raw` reached only through an `AttemptFact`,
`TraceFact`, or `AccountingFact`. Candidate 1 therefore has no operational-fact
path around an atom's source grant. If a caller separately supplies the same
document as an explicit derivation argument or assumption, its provenance is
that caller-supplied input; it is not reclassified as a source observation.

Let `pred*(p)` be the transitive predecessor plan-ID set of attempt plan `p`.
A trace is valid exactly when:

1. decision IDs, plan IDs, and completed attempt IDs recompute and are unique;
2. every decision `manifest` equals the digest of this acquisition manifest;
3. attempt order has already passed I0a's `(ordinal, id)` rule, and ordinals are
   unique;
4. each predecessor plan exists and has a lower ordinal;
5. the predecessor graph is acyclic;
6. every schedule decision uses the manifest schedule binding, declares
   exactly `pred*(p)`, and its output equals `p`'s proposal fields;
7. every selection decision uses the manifest selection binding, declares
   exactly `pred*(p) union {p}`, binds `p`'s outcome in its input snapshot, and
   its output equals the disposition kind and reason;
8. the stop decision uses the manifest stopping binding, declares exactly all
   plan IDs, and returns stop;
9. every schedule and selection decision is referenced by exactly one attempt,
   the stop decision is referenced once, and no decision is otherwise unused;
10. each outcome/disposition combination is legal under document 15;
11. every decision relation is re-executed and compared canonically; and
12. the attempts and generated artifacts fit the basis resource limits.

For zero attempts, the one stop decision declares the empty set and MUST return
stop. No schedule or selection decision exists.

Only `observed + include` may feed an atom. Excluded and censored observations
remain in the trace. Malformed and unavailable results are operational facts,
not refutations.

An unattested trace makes no accounting claim. A scoped accounting attestation
requires:

- an exact recorder grant whose recorder and scope match the claim;
- an exact semantic grant for its attestation verifier; and
- re-execution of the verifier yielding accepted.

The admitted fact is only that this recorder made an accepted accounting
assertion for this scope, checkpoint, and exception set. Because the verifier
does not receive a core-constructed trace commitment, it does not itself prove
trace completeness. It is not world completeness, semantic recall, correct
wall time, or target truth. `TraceFact` and `AccountingFact` make that bounded
record available to explicit missingness rules; absence alone is not a
premise.

## Graph admission

Graph admission requires:

- all acquisition and claim references to resolve under exact contracts;
- acquisition, claim, and node sets to be unique and canonically ordered;
- `claims` to equal the canonical set of basis targets and every node claim;
- `acquisitions` to equal the canonical set of every acquisition referenced by
  an atom or any attempt, trace, or accounting fact;
- node IDs to recompute;
- every atom to reference one resolved `observed + include` completed attempt;
- every semantic binding and every source consumed by an atom to have the exact
  required grant;
- every extractor and rule to re-execute with canonical output equality;
- every assumption ID to resolve in the basis;
- every fact reference to resolve under its tagged variant;
- every rule signature and modality output to match its descriptor; and
- all graph resource limits to hold.

Storage order is lexical node ID and has no dependency meaning. Acyclicity is
checked independently by deterministic depth-first search over evidence-node
premises, visiting roots and neighbors in lexical ID order. An evidence premise
may appear anywhere in the stored array.

Several atoms MAY reference the same completed attempt. Reuse remains explicit
in their qualified attempt references and never implies independent evidence.

The core has two stances, `support` and `refute`, and no ambient strength,
probability, cancellation, or source-independence rule.

## Provenance

For graphs admitted under the same environment, target set, trust basis,
assumption set, interpreter, and projection, define:

```text
G <=prov H  iff  node_ids(G) is a subset of node_ids(H)
```

Node IDs bind the complete atom or derivation identity preimage: kind, claim,
stance, extractor or rule binding, qualified attempt or premises, assumptions,
modality, payload, and derivation arguments. Relational admission also requires
every referenced premise, acquisition, claim, parameter, and grant. Therefore
exact node-ID inclusion is exact admitted-subgraph inclusion; no separate
field-wise matching relation can silently omit a dependency. It is a
provenance-information preorder, not an assurance order.

Candidate 1 uses exact content identity only. It contains no alias or semantic
lineage normalizer. Exact duplicate nodes already share one content ID and
cannot occur twice in canonical sets. Alias grouping and paraphrase relations
require a later language revision with their own falsification contract.

## Trust and assumptions

Candidate 1 grants are immutable, exact, and non-delegating:

- a `source` grant matches one source reference and one exact frame reference;
- a `recorder` grant matches one recorder reference and one exact accounting
  scope reference; and
- a `semantic` grant matches one semantics ID.

Every grant and trust-basis ID is recomputed. No evidence node, rule,
interpreter, projection, or semantic result may extend the caller-pinned
assessment basis. There is no
containment, delegation, expiry, supersession, or revocation operation.
Historical replay retains the historical basis reference; reevaluation uses a
new caller-selected basis reference.

Every consumed assumption is identified in a derivation or interpreter closure
and must exist in the basis. Authorization does not imply truth, competence,
independence, exchangeability, calibration, stable distribution, or complete
recording.

## Interpretation, projection, and dependencies

An interpreter reads only its admitted basis and graph and returns a typed
state. A projection reads only that state, targets, and parameters and returns
a non-empty finite label set for every target. The assessor—not either semantic
relation—computes dependency closures with the fixed dependency functions in
the bound specifications.

A closure names its root (`state` or `conclusion`), component pointer, facts,
assumptions, grants, exact semantic bindings, and direct document-input roots.
It is a full admission-justification closure, computed as follows:

1. use the exact `direct_inputs` root set fixed by the bound semantic
   specification and seed the facts used by the component;
2. recursively visit every referenced evidence node and every `FactRef` premise;
3. for every visited claim, record its claim-theory binding and semantic grant;
4. for an atom, record its qualified attempt, extractor binding and grant,
   source grant, schedule/selection/stopping bindings and grants, and every
   recorder/attestation binding and grant needed to admit the acquisition;
5. for a derivation, record its rule binding and grant and every listed
   assumption, then recurse through all premises, including transitive atom
   attempts;
6. for an attempt, trace, or accounting fact, record the same acquisition-policy
   and attestation dependencies; and
7. record the interpreter binding and grant for state; after projection, the
   assessor composes each conclusion closure from the state components named by
   the projection specification and records the projection binding and grant.

`direct_inputs` is not a flattened reference closure. Each entry is one root
designated by the semantic specification. Nested subject, proposition,
boundary, parameter, argument, payload, assumption statement/scope, and
acquisition documents remain recoverable through transitive tagged references
from the union of closure fields, but are not repeated as direct roots unless
the specification names them independently. All resulting collections are
deduplicated and canonically ordered. State closures exclude projection
bindings. Candidate builtins have fixed dependency functions and conformance
vectors; the assessor alone executes them. Closures do not enter a semantic
output contract or semantic fuel. This is completeness relative to admitted
justification, not a dynamic trace of every machine read. Candidate 1 admits no
opaque semantic extension.

A projection may intentionally map distinct states to the same label set. The
assessment outcome preserves the state document and closures alongside that
lossy projection; the projection itself does not preserve distinctions it
merges. An empty label set, fuel exhaustion, overflow, or malformed state is an
evaluation error, never abstention or evidence.

## Mandatory reference profile

The immutable artifacts under
`profiles/evidence/presence-candidate-1/` define the exact IDs, contracts,
parameters, and conformance vectors for the minimal profile.

For each canonical target claim, the presence interpreter forms:

```text
support_basis = set of support-node IDs for the target
refute_basis  = set of refute-node IDs for the target
```

Its state is:

| Support basis | Refute basis | State name |
|---|---|---|
| empty | empty | `neither` |
| non-empty | empty | `support_only` |
| empty | non-empty | `refute_only` |
| non-empty | non-empty | `both` |

State equality is byte equality of canonical target entries. Its information
preorder is componentwise set inclusion:

```text
(S1, R1) <=info (S2, R2) iff S1 subseteq S2 and R1 subseteq R2
```

Thus adding conflict can move upward in information while reducing resolution.
No count, vote, weight, confidence, or independence meaning is attached.

The cautious projection has label universe `{affirm, deny}`:

| State | Labels |
|---|---|
| `support_only` | `{affirm}` |
| `refute_only` | `{deny}` |
| `neither` | `{affirm, deny}` |
| `both` | `{affirm, deny}` |

`affirm` and `deny` are profile-relative labels, not unqualified truth or an
application action.

## Exact resource accounting

All additions are checked SafeInt additions. Resource counters mean:

| Counter | Exact quantity |
|---|---|
| `documents` | typed input document entries plus one generated state document when present; package wrapper, environment, and `AssessmentOutcome` document excluded |
| `total_document_bytes` | sum of UTF-8 byte lengths of `JCS(TypedDocument)` for those charged entries/documents |
| `attempts_total` | attempts across all referenced acquisitions |
| `graph_nodes` | atoms plus derivations |
| `graph_edges` | all `FactRef` entries in derivations, including non-evidence facts |
| `max_fan_in_observed` | maximum `premises` length of any derivation, or zero with none |
| `graph_depth` | longest evidence-premise path; atom or derivation with no evidence premise has depth one; empty graph has zero |
| `semantic_calls` | number of relation dispatches after all binding/input prechecks |
| `semantic_fuel` | sum of the candidate call-cost formula over each dispatched call's actual I-JSON output, including output later rejected |
| `state_bytes` | UTF-8 byte length of `JCS(state.value)`, or zero before state creation |
| `conclusion_labels_total` | sum of label-set cardinalities over all targets |

An `AttemptFact`, `TraceFact`, or `AccountingFact` contributes to graph edges
and fan-in but not graph depth. Selta structural verifier executions do not
count as evidence semantic calls; their work is bounded by the fixed schema
artifacts, schema-admission ceilings, and outer input ceilings, not by V1
`max_samples`.

Input documents remain charged even though an error package omits them. A
generated state is charged before insertion. If its typed-document digest
equals an input document digest, equal canonical bytes reuse the existing map
entry while the generation charge remains; unequal bytes stop in `interpret`
with `identity.collision`. The outcome document is excluded to prevent its
resource fields from measuring and hashing themselves. Before inserting a
concluded outcome, the assessor performs the same byte comparison against every
retained document with its digest. If an inner basis limit or generated
collision fails, the returned error package still contains only its outcome
document while retaining charged-work counters.

Candidate `builtin_total` relations always return one I-JSON value and do not
trap on a dispatch-ready call. A nonconforming contract-invalid output is
charged using that actual value before `evaluation.invalid_output`. A binding,
parameter, or input-domain failure occurs before dispatch and charges no call
or fuel.

## Deterministic errors

The phases and their order are:

```text
parse < schema < identity < relation < interpret < project < output
```

The [candidate error catalog](20-evidence-error-catalog.md) fixes the substage
schedule, cascade suppression, multiplicity, phase subject, pointer anchor,
message, charged-work commit points, and output-package contents. The first
failing substage is terminal; no implementation decides which dependent errors
are useful. Records are deduplicated and ordered exactly as that catalog
specifies. Human-readable messages are fixed ASCII protocol text, not
implementation-authored prose.

The typed outcome document is at most 1,048,576 canonical bytes. Document 20
fixes pointer normalization, record truncation, and a non-recursive minimal
`output.too_large` replacement, so the bound does not depend on input error
count, pointer length, or execution-set size. This outer output cap remains
available when an inner basis limit has failed.

Representative classes are:

```text
package.syntax                 package.duplicate_name
package.unknown_contract       package.contract_mismatch
package.missing_reference      package.unreachable_document
identity.noncanonical          identity.mismatch
identity.collision             relation.semantic_mismatch
relation.cycle                 relation.unauthorized
resource.outer_exceeded        resource.basis_exceeded
evaluation.fuel_exhausted      evaluation.invalid_output
```

The conformance corpus fixes the complete code, pointer, message, and
resource output for every vector.

## Laws

Every conforming candidate-1 implementation satisfies:

1. **Closed admission.** Unknown fields, revisions, contracts, and references
   fail before interpretation.
2. **Exact replay.** Equal canonical package, caller-supplied
   `ExpectedBasisRef`, environment artifacts, resolver results, implementation
   artifacts, and limits produce a byte-equal outcome.
3. **Semantic replaceability.** Conforming implementations of the same bound
   semantics produce equal state, closures, conclusions, and usage; execution
   artifact records may differ.
4. **Provenance reconstruction.** Every projected component has recoverable
   transitive facts, assumptions, grants, bindings, and direct inputs.
5. **Duplicate safety.** Duplicate IDs and duplicate members of canonical sets
   are rejected; repetition cannot change presence state.
6. **No inferred independence.** Multiplicity of executions, actors, models,
   prompts, documents, or aliases creates no independence premise.
7. **Conflict preservation.** Support and refutation remain separately
   inspectable in state before projection.
8. **Polarity explicitness.** Every evidence node states its stance; absence is
   neither stance.
9. **No hidden modality coercion.** Modality changes only through a typed rule.
10. **Operational separation.** Outcomes become claim evidence only through a
    validated extractor or derivation; operational facts do not expose nested
    source payloads.
11. **Assumption visibility.** Every consumed assumption is present in the
    basis and dependency closure.
12. **Authority non-escalation.** No assessment computation extends the grants,
    assumptions, semantics, targets, or limits in the caller-pinned basis.
13. **Boundedness.** Finite input in the outer domain terminates through
    built-in total relations within declared limits or returns an error.
14. **Canonical identity.** Distinct content identities remain distinct; the
    core performs no alias or paraphrase collapse.
15. **Non-empty projection.** Every successful target projection has at least
    one label.

Source competence, truth of assumptions, calibration stability, unrestricted
paraphrase equivalence, world completeness, and product value are empirical or
theory-relative claims, not generic laws.

Assurance, calibration, and amplification statements are ordinary typed claims
and derivations under separately specified profiles. Candidate 1 gives them no
ambient certification category.

## Privacy boundary

Content hashes are public equality tests and can reveal low-entropy values by
guessing. Private payloads SHOULD use a typed sealed or blinded representation
whose contract states leakage and verification behavior. Encryption, HMAC,
salt, key custody, disclosure, and key revocation remain profile concerns.

Sealing content does not hide document count, graph shape, roles, causal edges,
or ordinals. Native usernames, paths, hostnames, process IDs, clocks, and
provider secrets do not enter the generic language unless an application
deliberately wraps them in a typed value and accepts the leakage.

## Platform boundary

The judgments use canonical bytes, typed references, causal ordinals, and
abstract counters. Given equivalent normalized observations and a conforming
adapter contract, admission, interpretation, and projection are
platform-independent. Adapters may legitimately observe different external
process or storage behavior on different systems; those differences enter as
different typed observations, not hidden changes to the calculus.

No operating system is supported until the same conformance corpus passes in a
real environment for that system. Windows remains an architectural target, not
a current support claim.

## S2 semantic gate

The durable Rust reference crate and every product integration remain blocked
until S2 closes. S2-only conformance models are instead permitted and required:
they are isolated executables with no public API, no inbound production
dependency, and no authority beyond producing sealed predictions for the
candidate corpus. They MUST be implemented independently of one another, using
only these public artifacts, and MUST NOT share candidate semantic, parser,
identity, or error code. They are disposable test evidence, not the future
reference runtime.

For cross-model comparison, the arbiter first validates each complete returned
package and all of its identities independently. It then constructs this
non-serialized conformance projection from the resolved outcome:

```text
ConformanceProjection {
  revision,
  status,
  environment,
  basis?: DocumentRef,
  state?: { reference: DocumentRef, value: JSON },
  closures?: [DependencyClosure],
  conclusions?: [{ claim: DocumentRef, labels: [DocumentRef] }],
  resources: ResourceUsage,
  errors?: [{ phase, code, path, message }],
  semantics: [SemanticsId]
}
```

`semantics` is the canonical semantics-ID projection of `executions`.
Implementation digests, the outcome document, its digest, and the package root
are excluded because truthful implementation identities may differ. No other
field is removed or normalized. A concluded outcome supplies `basis`, `state`,
`closures`, and `conclusions` and omits `errors`; an error outcome supplies
`errors` and omits those concluded-only fields. This projection exists only for
S2 conformance; it is not candidate DSL, identity input, or product output.

S2 closes only when:

- all schemas named in document 15 exist and admit through stable Selta;
- the environment, presence profile, labels, error catalog, and complete
  package/outcome vectors have immutable computed IDs;
- two independently authored conformance models implement the complete
  assessment judgment from public artifacts and held-out inputs alone;
- every law has positive and falsifying fixtures;
- identity, collection order, resources, trust, privacy, and errors are
  independently reproducible;
- the stable Selta 0.1 boundary remains unchanged and a future legacy adapter
  can consume the candidate without reimplementing V1; exact legacy
  differential equivalence remains the deferred L4 gate;
  and
- independent formal and compatibility reviews have no unresolved blocker.

**Current status:** the presence/error corpus does not yet provide executable
positive and falsifying fixtures for every law, and two independent complete
conformance models do not yet exist. The S2 gate therefore remains open; this
candidate does not authorize a durable Rust reference crate or product
integration yet.
