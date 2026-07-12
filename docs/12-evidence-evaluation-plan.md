# 12 — Evidence research and evaluation plan

> **Status: active gated research plan; S0 and S1 complete, S2 candidate under
> review.** Selta 0.1 remains authoritative. Completing one stage does not
> authorize the next, and no candidate interface is stable.

## Objective

Determine whether Selta can compose fallible semantic assessments without
turning repetition, recursion, or agreement into unjustified confidence. The
work must produce one of two useful outcomes:

1. a small, law-governed evidence DSL with measurable value and a conservative
   path from Selta 0.1; or
2. a documented negative result that bounds what Selta should not claim or
   implement.

The objective is not to add more verdict states around individual failure
cases. It is to identify the information and assumptions needed for an
epistemic composition to be valid.

## Working rules

1. **DSL before implementation.** Every durable input, output, inference,
   trust declaration, and projection must first be described as a versioned Selta
   schema and accompanied by denotational and operational semantics.
2. **One legacy authority.** The 0.1 engine and protocol remain the sole stable
   behavior throughout research and reference stages.
3. **Pure core.** A candidate reference implementation, if admitted, starts as
   a small pure crate. It contains no model client, prompt, HTTP server,
   scheduler, process supervisor, or product policy.
4. **Explicit assumptions.** Source competence, calibration, independence,
   exchangeability, and trust are data. Execution multiplicity never implies
   any of them.
5. **Falsification before promotion.** Counterexamples and competing baselines
   are designed before feature code.
6. **Real evaluation.** Scripted fixtures are useful for laws and fault
   injection, but evidence of semantic value requires pinned real assessors
   and real inputs. Mock judges do not count as empirical validation.
7. **Matched comparisons.** Compare methods at matched input, model revision,
   information access, token/call budget, latency class, and abstention
   coverage. More computation is not itself an improvement.
8. **Negative results survive.** Failed hypotheses, prompts, dependence tests,
   and calibration results remain versioned research artifacts.

## Stage plan

### S0 — Isolate and freeze

**Artifacts**

- an isolated research worktree and branch;
- the exact Selta 0.1 source, language, meta-validator, and protocol baseline;
- the frozen public-contract inventory in
  [11-compatibility-and-versioning.md](11-compatibility-and-versioning.md); and
- an external dependent notice whose immediate action is `none`.

**Exit gate**

- the stable checkout is unchanged;
- existing path consumers still resolve the frozen checkout; and
- no experimental document promises a stable API.

### S1 — State the problem and research ledger

Build and maintain the primary-source ledger in
[13-evidence-research-ledger.md](13-evidence-research-ledger.md) for the limits
and candidate tools: judgment
aggregation, selective prediction and abstention, calibration, correlated
ensembles, provenance and evidence algebras, truth-maintenance, proof-carrying
results, and optimization against learned evaluators.

For each imported result, record its domain, assumptions, conclusion, and the
reason it does or does not transfer to opaque semantic assessors. A citation is
not a design law until that transfer argument is written.

**Exit gate**

- every claimed boundary result in
  [10-evidence-and-decision.md](10-evidence-and-decision.md) has a primary
  source or a self-contained counterexample;
- terms such as evidence, information, trust, claim, incompatibility,
  dependence, soundness, assurance, conclusion, and abstention have testable
  meanings; and
- at least one minimal counterexample defeats each naive rule under study:
  majority, unanimity, recursive checking, deeper retries, and best-of-N
  selection.

### S2 — Specify the calculus and DSL

Write a normative candidate specification before Rust types. At minimum it
must define:

- canonical claim identity and polarity;
- an acquisition manifest binding selection frame, attempted sources,
  adaptive policy, censoring and inclusion rules, planned budget, stopping
  rule, channel declaration, and any separately scoped accounting assertion;
- an attempt trace with a scoped accounting attestation that preserves
  successes, malformed results, refusals, timeouts, cancellations, and budget
  exhaustion without claiming the attestation proves completeness;
- evidence atoms, typed payloads, source revision, observation lineage, and
  permitted context projection;
- content-addressed derivations with named rules and recoverable premises;
- trust bases and assumption sets, including acquisition-recorder authority;
- interpreter and conclusion-projection identities;
- a replayable conclusion basis binding all relevant digests; and
- bounded, deterministic validation and interpretation failure behavior.

The specification must also define the claim language and canonical equality,
any profile-specific incompatibility or negation relation, graph typing and inference judgments, an
evidence-information preorder, conclusion universe, meanings of singleton,
non-singleton, and empty conclusion sets, and the preservation obligations of
interpreters and projections. Epistemic conclusions and application actions
must not share one ambiguous `DecisionSet` type.

Every serialized artifact is admitted by a versioned Selta schema. The DSL
must distinguish malformed evidence, unavailable evidence, evidence against a
claim, and conflicting evidence. It must not encode confidence as an
unexplained scalar.

Candidate laws are tested algebraically and with generated finite graphs:
provenance preservation, replay, exact duplicate rejection, canonical set
ordering, no inferred independence, conflict and polarity preservation,
polarity symmetry, explicit coercion, operational separation, acquisition
accountability, bounded evaluation, and legacy conservativity. Paraphrase
resistance remains an empirical test because the graph calculus cannot assume a
general semantic-equivalence decider.

**Exit gate**

- the grammar, static rules, dynamic semantics, equality, and canonical
  encoding are reviewable without reading implementation code;
- each advertised amplification rule names sufficient assumptions and has a
  proof sketch or a bounded claim that can be experimentally falsified; and
- two independent implementations could produce compatible artifacts from the
  specification alone.

### S3 — Build a pure reference model

Only after S2, consider an experimental `selta-epistemics` crate. It should
contain values, canonicalization, admission, finite graph construction, pure
interpreters, and pure conclusion projection. It should not modify `selta-core`
or protocol 1.

Use property tests, exhaustive small-graph checks where tractable, and a second
small executable model of the semantics to detect implementation agreement on
the same mistaken code path. If the chosen algebra supports mechanization,
formalize the central no-amplification and conservativity claims; do not label
ordinary Rust tests as proofs.

**Exit gate**

- all S2 laws pass across the reference and independent models;
- malformed, cyclic, oversized, and unknown-revision graphs fail closed;
- graph admission and interpretation have explicit resource bounds; and
- canonical encoding, domain-separated hashing, alias equivalence, downgrade,
  collision handling, and adversarial graph/payload/rule inputs pass security
  tests;
- privacy review covers leakage through content hashes, source IDs, lineage,
  context commitments, and attempt timing;
- caller-pinned immutable assessment-basis references distinguish historical
  replay from re-evaluation under a new basis; and
- the stable workspace suite is unchanged.

### S4 — Run falsifying case studies

The evaluation corpus has several strata because no single task exposes all
failure modes.

| Stratum | Purpose | Authority |
|---|---|---|
| Deterministic constraints | Verify exact legacy projection and error separation | Mechanically computed result |
| Witness-bearing semantics | Test whether new witnesses really amplify assurance | Independent checker or executable counterexample |
| Ambiguous classification | Measure disagreement, abstention, and policy effects | Multiple blinded human annotations plus adjudication record |
| Correlated assessors | Expose false independence across prompts, samples, and model aliases | Controlled shared-lineage experiments |
| Adversarial optimization | Detect evaluator gaming under retries and best-of-N search | Hidden holdout authority and post-search audit |
| Missing and degraded service | Test explicit missingness rules without operational leakage | Injected transport and resource faults |
| Adaptive acquisition | Detect cherry-picking, optional stopping, and feedback-dependent sampling | Hidden authoritative channel log |

Baselines include one assessor, repeated identical assessment, majority and
unanimity rules, Selta 0.1 voting, a selective-prediction baseline, and any
claimed evidence interpretation. Results are stratified by task, source,
lineage, and difficulty rather than reported only as a pooled average.

Use accuracy only where a defensible reference answer exists. Depending on the
stratum, report risk at fixed coverage, coverage at fixed risk, calibration
error and proper scoring rules, conflict-retention rate, false-amplification
rate, operational-error leakage, cost, latency, and reproducibility. For
subjective labels, preserve the annotation distribution and adjudication basis
instead of declaring consensus to be ground truth.

Real-assessor runs pin provider, model revision or snapshot, parameters,
prompts, tool permissions, context projection, timestamp, and lineage. The
evaluation harness may contain provider adapters, but provider concepts do not
enter the core DSL.

Before any run, the study protocol preregisters task-specific minimum coverage,
maximum selective risk, non-inferiority margins for unaffected strata, sample
size or stopping rule, confidence-interval method, matched-cost definition,
development/holdout split, and rejection threshold. These values cannot be
chosen after inspecting the holdout result.

**Exit gate**

- no advertised assurance increase can be reproduced by duplicating one
  evidence identity, issuing aliases, paraphrasing, reordering, or disguising
  common lineage;
- cherry-picking, censoring, and adaptive stopping are detected relative to the
  acquisition manifest and channel-attested attempt trace;
- conflicts and abstentions remain observable through projection;
- at matched cost and coverage, a candidate shows a predeclared improvement on
  at least one task class without hiding a material regression on another;
- prompt or policy tuning uses a development split and the conclusion survives
  a held-out run; and
- all failures and null results are reported.

Failure of this gate may narrow or terminate the feature. It does not justify
adding application-specific exceptions to the core.

### S5 — Decide the interface, not merely its Rust shape

If S4 succeeds, write an interface RFC comparing a separate `assess` lane, a
capability-negotiated protocol 2, and a separately versioned evidence report.
The RFC must specify the DSL, authorities, capability negotiation, resource
model, privacy treatment, host compatibility matrix, and version binding.

The current preference is a separate assessment lane because it preserves the
meaning of `verify`; it is not yet a decision.

**Exit gate**

- the selected interface is strictly opt-in;
- protocol-1 hosts and legacy reports retain exact behavior;
- untrusted hosts cannot self-assert proof status, independence, or trust; and
- the security, privacy, compatibility, and rollback analyses are accepted.

### S6 — Shadow integration

Interpret already recorded legacy calls, or generate evidence beside existing
verification with a separately allocated quota, trace, and lifecycle. Shadow
work cannot affect the authoritative call sequence, verdict, retry behavior,
budget, cache, report, or host lifecycle. Run the legacy differential corpus
and consumer-specific corpora. Classify every difference as expected, defect,
or unresolved before promotion.

**Exit gate**

- the legacy adapter reproduces every frozen result;
- the public API, grammar, default builtin inventory, and protocol-1 bytes are
  unchanged for legacy users;
- shadow execution cannot consume production authority or budget unless
  explicitly allocated; and
- disabling the feature restores the exact stable path.

### S7 — Offer an explicit migration candidate

Publish a release-candidate notice containing exact commits, source manifests,
schema and semantics revisions, changed symbols, new DSL constructs, known
behavioral differences, conformance evidence, adapter, and rollback. Each
consumer makes its own acceptance decision and updates all pins atomically.

Migration policy never changes the canonical checkout, dependency source, or
pins automatically, and there is no silent default switch. A live path
dependency follows edits to its target, so isolation and source attestation are
required until explicit acceptance.

## Promotion and falsification register

The following are release blockers, not aspirational qualities.

| Property | Falsifying observation |
|---|---|
| Independence safety | Repeated or relabelled common-lineage evidence raises default assurance |
| Acquisition integrity | Omitted attempts, adaptive stopping, censoring, or post-selection cannot be detected from the bound basis |
| Conservativity | The legacy adapter changes any frozen 0.1 result outside a declared serialization equivalence |
| Conflict preservation | An admitted conflict disappears before an explicit projection policy |
| Assumption visibility | A conclusion relies on competence, trust, calibration, or dependence not bound into its basis |
| Authority safety | A source can grant itself a stronger modality or trust role |
| Replay | The same complete basis can yield another conclusion without a revision change |
| Boundedness | Valid finite input can trigger unbounded interpretation or derivation expansion |
| Identity security | Alias, canonicalization, hash-domain, downgrade, or collision behavior can change the conclusion basis silently |
| Privacy | A supposedly hidden payload is recoverable or linkable beyond its declared access policy through metadata or commitments |
| Empirical value | Apparent gain disappears under matched cost, fixed coverage, or held-out evaluation |
| Consumer isolation | Stable users can observe experimental behavior without opting in |

Any row falsifies promotion. The response is to repair the theory and repeat
the held-out test, narrow the advertised domain, or stop the feature. It is not
to waive the property after observing the failure.

## Questions deliberately deferred beyond candidate 1

- richer evidence algebras beyond the minimal exact-identity presence profile;
- theory-specific incompatibility and negation operations;
- cryptographic sealed/blinded source and payload representations;
- concrete adaptive acquisition policy profiles;
- any assumption-bearing amplification rule beyond ordinary typed derivation;
- probability or calibration profiles;
- promotion to a stable assessment interface or protocol version; and
- the threshold for ending the product direction after a negative S4 result.

Documents 15 through 20 resolve only the candidate-1 grammar, semantics,
reference profile, error contract, and compatibility boundary needed for S2.
The remaining questions are later gated outputs, not details to guess during a
reference implementation.
