# 10 — Evidence and decision: research note

> **Status: non-normative research track.** This document does not change the
> schema language, verification semantics, host protocol, report contract, or
> compatibility promises of Selta 0.1. The stable baseline is recorded in
> [11-compatibility-and-versioning.md](11-compatibility-and-versioning.md).

## Problem

Selta 0.1 models a verifier as:

```text
verify(value, context) -> pass | fail(delta) | error
```

This is appropriate for a deterministic constraint whose accepted set is
already defined. It is not a complete epistemic model for an opaque semantic
assessor. Repeating, composing, or recursively checking fallible judgments can
measure the assessor's behavior without establishing that the behavior tracks
the intended semantic fact.

The foundational question is **epistemic composition**:

> Under what declared assumptions does composing fallible assessments
> increase, preserve, or decrease assurance about a claim?

No generic `all_of`, majority vote, or recursive verifier answers that
question. A composition may claim increased assurance only when it carries a
named amplification argument whose assumptions are explicit.

## Boundary results

This research begins from four limits rather than from a new verdict enum.

1. **No information-free amplification.** A transformation of an existing
   observation cannot add information about a latent fact merely by running
   another judge. A later stage must add evidence, expose a truth-preserving
   decomposition, or rely on an explicit competence and dependence model.
2. **No universal semantic decider.** Selta is intended to regulate values
   produced by general programs. Non-trivial semantic properties of arbitrary
   programs are not all decidable by a sound, complete, terminating procedure.
3. **No universal judgment aggregator.** For logically connected judgments,
   no aggregation rule simultaneously satisfies every natural coherence,
   anonymity, systematicity, completeness, and unrestricted-domain condition.
4. **No free optimization against a proxy.** Once generation or repair observes
   verifier feedback, the system searches for verifier-compatible outputs.
   More search can amplify a verifier's blind spots instead of truth.

Selta cannot manufacture an assumption-free truth oracle. It can make the
evidence, assumptions, derivation, trust boundary, and conclusion projection
explicit.

## Research thesis

The candidate foundation separates acquisition, evidence, interpretation, and
conclusion:

```text
declared acquisition plan
    -> attempt trace with scoped accounting attestation
    -> typed evidence derivation graph
    -> interpretation under trust base T and assumptions A
    -> conclusion projection P
    -> justified conclusion set
```

In notation:

```text
record(plan, attempts, stopping_basis) -> AcquisitionTrace
admit(AcquisitionTrace) -> EvidenceGraph
interpret(graph, T, A, interpreter_revision) -> EpistemicState
conclude(EpistemicState, projection_revision) -> ConclusionSet
```

The evidence graph is intended to be a free, information-preserving syntax.
Candidate 1 uses exact content identity and no alias quotient. Boolean truth,
K3, provenance semirings, bilattices,
probabilities, credal bounds, and reliability models are candidate
interpretations or projections over it. None is silently the universal evidence
domain. Cost and trust remain bound inputs rather than truth values.

## Minimal candidate vocabulary

The vocabulary is deliberately smaller than a catalogue of uncertainty cases.
Exact DSL names and formal judgments remain undecided.

### Acquisition manifest and trace

A versioned acquisition manifest defines how observations may be requested and
selected. It binds the selection frame, source and prompt/configuration
revisions, permitted context, planned budget, adaptive policy, censoring and
inclusion rules, and stopping rule. The resulting immutable trace records every
declared attempt in order or causal partial order: successes, malformed results,
timeouts, refusals, cancellations, and budget exhaustion.

An evidence graph is not observationally complete merely because its internal
provenance is complete. Each atom must point to a recorded attempt. Detecting
omitted or post-selected observations additionally requires an admitted
exclusive channel or external accounting mechanism whose exact promise is
stated; a manifest alone is insufficient. Precommitment to one plan cannot
reveal secret parallel queries, and a trace supplied by an untrusted producer
cannot prove its own completeness. The trace may therefore carry a scoped
accounting attestation without claiming that authentication proves trace or
universal completeness. Secret payloads may use access-controlled commitments,
but privacy cannot be achieved by silently deleting attempt lineage;
low-entropy payload hashes can themselves leak information.

### Claim

A canonical, versioned proposition about a value, trace, or artifact. A claim
has stable identity independent of the assessor that discusses it.

### Evidence atom

One admitted observation concerning a claim. It binds:

- the claim and polarity (support or refutation);
- source identity and semantic revision;
- its acquisition attempt and observation lineage, including common origins;
- a typed payload;
- the context projection the source was permitted to observe; and
- content identity.

An atom is not a verdict. An unavailable source first produces an operational
attempt fact, not negative claim evidence. A later inference rule may interpret
missingness only when it declares the acquisition model and assumptions that
make missingness informative; the original attempt fact remains recoverable.

### Derivation

A content-addressed node that derives evidence for a claim from prior evidence
under a registered inference-rule revision. Its premises remain recoverable.
The rule identifies its assumptions; it does not acquire authority from the
host that requests it.

### Evidence graph

An immutable, finite, acyclic graph of atoms and derivations. It preserves both
support and refutation. Distinct execution IDs do not imply independent
evidence: samples can be distinct atoms while retaining common source, prompt,
settings, input, and batch lineage.

### Trust base and assumptions

Versioned values that state which sources, acquisition recorders, and inference
rules are admitted and which statistical, independence, calibration, or
environmental assumptions an interpretation may use. Trust is input to a
conclusion, never an ambient fact.

### Interpreter and conclusion projection

A pure interpreter maps an admitted graph into a declared epistemic domain. A
pure projection maps that state into a set of justified conclusion labels from
a declared universe. A singleton is resolved; a non-singleton remains
unresolved; an empty set is a policy or evaluation error, not a conclusion.
Application actions and permissions are outside this epistemic result.

The final basis binds the acquisition manifest and trace, graph, trust base,
assumptions, interpreter, projection, and their revisions and digests. S2 must
define the claim language and incompatibility relation, graph typing and
inference judgments, an evidence-information preorder, and the preservation
obligations of each interpreter and projection. Until then, `EpistemicState`
and `ConclusionSet` are requirements on the specification, not public types.

## Required laws

Any candidate calculus must satisfy these laws before a stable or public API is
selected. Isolated conformance surfaces may exist only to test the laws.
These are non-normative research obligations; document 16 is the sole numbered
candidate-1 law inventory and maps or narrows each obligation operationally.

1. **Provenance preservation.** The basis of every interpreted result remains
   reconstructable from the graph.
2. **Replay.** Equal acquisition basis, graph, trust base, assumptions,
   interpreter, and projection produce the same conclusion set.
3. **No implicit amplification.** Under a versioned lineage-equivalence
   relation, a graph morphism that only copies, aliases, reorders, or relabels
   existing observations cannot increase default assurance. New identifiers do
   not evade this law. Resistance to semantic paraphrase is an empirical test,
   not assumed decidable by the formal graph calculus.
4. **No inferred independence.** Independence is an admitted assumption, not a
   consequence of multiple requests or actors.
5. **Conflict preservation.** Support for incompatible claims cannot collapse
   silently into absence, a majority, or one selected critique.
6. **Polarity symmetry.** Support and refutation can both carry typed evidence;
   success is not evidence-free merely by convention.
7. **No hidden coercion.** Testimony, a statistical sample, and a mechanically
   checked proof are different epistemic modalities. Conversion requires an
   explicit admitted rule.
8. **Operational separation.** Timeout, malformed transport, missing settings,
   and budget exhaustion remain operational facts unless an explicit admitted
   rule converts their missingness under a declared acquisition model. The
   conversion never erases the attempt facts.
9. **Bounded evaluation.** Finite admitted graphs and total interpreters
   terminate. Resource depth is not an assurance measure.
10. **Stable-boundary non-interference.** Candidate evidence work does not
    change or reimplement Selta 0.1 sampling, K3, reports, or protocol behavior.
    A future legacy profile may represent those results, but byte-level
    differential equivalence belongs to the later L4 gate.
11. **Acquisition accountability.** Every attempt, its outcome, and its
    inclusion, exclusion, or censoring decision is reconstructable relative to
    the declared acquisition manifest, adaptive choices, stopping rule, and an
    admitted exclusive or attestable channel. Completeness is claimed only for
    the scope controlled by that authority.
12. **Canonical identity.** Canonical encoding, hash domain separation, and
    lineage equivalence are revisioned and reject ambiguity or downgrade.

## Operational and epistemic axes

Operational properties and epistemic status are orthogonal. Operational
attributes form a product, not the alternatives of one enum.

```text
operational: { repeatability, cacheability, effects, availability, cost, ... }
epistemic:   proof | observation | sample | testimony | derivation | ...
```

A temperature-zero model may be operationally deterministic without producing
a proof. A theorem prover reached over an unreliable transport may be
operationally fallible while a successfully checked proof remains decisive
under its trust base. Selta's existing determinism declaration therefore
remains an execution and caching concern; it must not imply truth authority.

The epistemic list above is illustrative, not a closed enum. The intended core
is a typed term and rule system whose admitted evidence domains declare their
own laws.

## Amplification arguments

A derivation may claim increased assurance only through a registered rule with
declared premises and assumptions. Candidate classes include:

- independent repetition under a calibrated sampling model;
- a mechanically checked witness or counterexample;
- a proved truth-preserving decomposition;
- a complementary source with a declared dependence model;
- adversarial cross-examination with a stated soundness argument; or
- acquisition of a new external observation.

This is not a closed list of product behaviors. Each item is an example of the
same obligation: an assurance increase requires an explicit derivation rule,
not merely more computation.

## Relative soundness

The strongest honest general claim is relative and deliberately modest:

> Given an admitted evidence graph, trust base and inference rules `T`,
> assumptions `A`, and an interpreter and projection sound under those
> assumptions, Selta deterministically reproduces the conclusion and its
> admitted derivation basis.

This is derivational reproducibility relative to included premises. It is not
observational completeness, a proof that the acquisition trace contains every
relevant fact, or a claim that opaque testimony is true. A result should be
understood as:

```text
concluded(value | acquisition, graph, trust base, assumptions, interpreter, projection)
```

not as an unqualified fact detached from its basis.

## Relationship to Selta 0.1

The research does not discard the existing system.

- Structural checking remains a deterministic type layer.
- K3 remains a useful legacy conclusion algebra for strict refinement
  verification.
- `Delta` remains the legacy negative explanation contract.
- `VotePolicy` remains one legacy conclusion projection over recorded sample
  outcomes. Any stronger assurance interpretation must separately declare its
  competence and dependence assumptions.
- `depth` remains a termination and resource bound for legacy delta checking.
- Existing `verify` and protocol-1 behavior remain frozen while the research is
  experimental.

The central correction is that K3 is not treated as Selta's universal evidence
domain and verifier sampling is not treated as semantic truth.

## Scope

In scope for this research:

- generic evidence and derivation identity;
- provenance and dependence preservation;
- evidence-domain and inference-rule admission;
- pure interpretation and conclusion projection;
- replayable conclusion bases; and
- compatibility with strict structural verification.

Out of scope:

- LLM provider clients and prompts;
- retry and repair loops;
- agent, mission, memory, scheduling, or queue concepts;
- application-specific labels or policies;
- a universal probability model; and
- automatic claims that deeper checking is more trustworthy.

An abstraction belongs here only if it can constrain both a conventional
program and a human or model assessor without naming either product runtime.

## Open questions

1. What is the smallest canonical graph grammar that preserves derivations
   without embedding one evidence theory?
2. Which algebraic interface is sufficient for combination, polarity,
   projection, and recursion: provenance terms, valuation algebras,
   justification terms, a product construction, or a smaller custom core?
3. How are negation and conflicting evidence represented without making one
   interpretation canonical?
4. Which source and lineage identities are required to prevent false
   independence while preserving privacy?
5. How does a rule prove or declare its amplification law, and which claims are
   enforceable by admission rather than review?
6. How are acquisition manifests, adaptive stopping, failed attempts, and
   privacy-preserving commitments represented without enabling post-selection?
7. Which trust revocation or supersession events affect historical replay, and
   which trigger a current re-evaluation under a new basis?
8. How are interpreter and projection semantics bound to immutable catalog
   versions?
9. Can the legacy verifier be expressed conservatively without duplicating the
   verification engine?
10. What finite graph and payload bounds preserve useful evidence while making
   denial-of-service behavior impossible?

## Research roots

The initial primary-source results, their transfer arguments, and their limits
are recorded in
[13-evidence-research-ledger.md](13-evidence-research-ledger.md). None is
adopted as Selta's final evidence semantics by this note.
