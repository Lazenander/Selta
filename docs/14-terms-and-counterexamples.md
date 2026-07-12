# 14 — Epistemic terms and counterexamples

> **Status: S1 research contract.** This document fixes the vocabulary and
> countermodels that an S2 language must respect. It is not part of Selta 0.1
> verification semantics and does not add a public interface.

## Purpose

The evidence track needs a small vocabulary whose claims can be falsified. It
must not hide uncertainty behind additional verdict names or treat a practical
heuristic as a universal evidence theory.

The generic core therefore records provenance and derivation. Probability,
voting, calibration, reliability weighting, argumentation, and K3 are explicit
interpretations or projections, never ambient meanings of the graph.

## Terms

### Claim and claim theory

A **claim** is a canonical proposition under a named claim-theory revision. It
binds the subject or artifact it concerns and the state or time boundary at
which its truth may be evaluated. Its identity is independent of the assessor
that comments on it.

A **claim theory** defines an idempotent canonicalization and equality as byte
equality of canonical claims. It may additionally define a symmetric,
irreflexive incompatibility relation `c # d` or partial negation, with their
domains and algebraic laws declared by that theory. Selta does not infer that
evidence refuting `c` supports a claim `not(c)` unless an admitted rule licenses
the conversion.

### Acquisition scope

An **acquisition scope** is a mechanically described boundary over one channel,
query or selection frame, state interval, and checkpoint or watermark. It is
not a vague semantic category.

A **scoped accounting attestation** records a recorder's or channel's assertion
that it accounted for every attempt matching that scope, subject to listed
exceptions. Authentication can establish who made the assertion; it does not
by itself prove trace or world completeness. Open world is the default.

Consequently:

```text
not_observed(c) != observed_refutation(c)
complete(raw acquisition) != complete(semantic recognition)
authorized_to_record(scope) != complete_recording(scope)
```

Event time, arrival time, observation time, and integration time are distinct
when a policy uses them. A trace may omit these clocks when they are irrelevant;
it may not silently treat one as another.

### Manifest, trace, and attempt

An **acquisition manifest** predeclares the eligible frame, sources, permitted
context, policies, resource budget, randomness commitments where applicable,
inclusion and censoring rules, stopping rule, and channel. A recorder and scope
appear only when a scoped accounting attestation is supplied.

An **attempt trace** is an immutable causal record. For every declared attempt
it records the request commitment, source and configuration, outcome, and the
later inclusion, exclusion, or censoring disposition. A successful but
excluded result remains present. A final trace has no pending or unexplained
recorded disposition. An attestation may assert a broader accounting scope; the
trace alone cannot prove that unrecorded attempts do not exist.

Precommitment proves that a plan existed. It cannot reveal a secret parallel
channel. Trace completeness is therefore always relative to an admitted
exclusive or externally attestable acquisition boundary.

### Observation and evidence

An **observation** is a recorded assertion or outcome attributed to an
acquisition recorder or channel. Appropriate attestation may establish that a
source emitted it; the trace alone does not. In neither case is it yet a fact
about the target claim.

An **evidence atom** is an admitted extraction from an observation that bears a
declared stance toward a claim. It retains the attempt, source, configuration,
context, extraction rule, payload contract, and recoverable recorded
provenance. An atom is
not a final verdict and cannot self-assert trust, independence, or proof status.

An **operational fact** records timeout, refusal, malformed transport,
cancellation, or resource exhaustion. It remains disjoint from claim evidence.
A rule may interpret missingness only under an explicit acquisition model and
must retain the operational premise.

### Derivation and modality

A **derivation** applies one registered, revisioned rule to recoverable premises
and explicit assumptions. It produces a stance toward a claim and states its
evidence modality. The rule—not its caller—defines its signature and authority
requirements.

A **modality** states what kind of reason is carried, such as testimony,
sample, checked witness, or derivation. The core does not define a closed list
or an ordering between modalities. Conversion requires an explicit rule.

### Trust, authorization, assumption, and calibration

These are distinct:

- **authorization** permits a source, recorder, rule, interpreter, or projector
  to participate in a declared scope;
- **trust** is a caller-supplied basis admitting those authorities for one
  assessment;
- an **assumption** is a proposition under which an interpretation is
  conditional; and
- **calibration evidence** is evidence about a source's behavior on a declared
  population and period.

Authorization does not imply truth. The complete assessment basis is
caller-pinned input to replay, not a property a source or package can alter in
its own output. Historical replay uses its historical basis reference;
reevaluation under a newer basis creates a new result without mutating the old
one.

### Provenance information

For valid graphs `G` and `H`:

```text
G <=prov H
```

means that `H` contains a provenance-preserving embedding of `G`.

Candidate 1 uses exact content identity and defines no generic alias or
paraphrase equivalence. A later profile may introduce a named, falsifiable
lineage relation, but it is not required by the minimal presence calculus.

The provenance relation does not mean "more certain". Additional conflicting evidence can be
more informative while making a conclusion less resolved. The generic core has
no scalar assurance order. Each optional interpreter declares its own domain,
equality, information order, and preservation laws.

### Epistemic state and conclusion

An **interpreter** is a pure, revisioned, resource-bounded map from an admitted
basis to an **epistemic state**. The state exposes the premise closure used for
each component.

A **conclusion projection** maps that state to a non-empty subset of a finite,
revisioned conclusion universe:

- one label is resolved;
- more than one label is unresolved; and
- an empty set is an evaluation error.

This is epistemic output, not an application permission or action. Abstention is
a valid unresolved projection, not an operational failure.

### Relative soundness

**Derivational replay** means that the same canonical acquisition basis, graph,
trust base, assumptions, interpreter, projection, and resource profile produce
the same result.

**Relative soundness** is a theorem or empirical claim about a named
interpreter or rule under explicit assumptions. It never means that Selta made
an opaque assessor true. The graph can be derivationally complete relative to
its recorded premises while the acquisition remains observationally incomplete
outside its declared scope.

## Minimal countermodels

Each case defeats a generic rule. An implementation that produces a decisive
answer may do so only through additional, named premises.

### C1 — verifier cascades increase false rejection

Let an acceptable answer face `d` independent semantic judges, each with
false-rejection probability `a > 0`, combined by `all_of`. Its pass probability
is:

```text
(1 - a)^d
```

and false rejection is `1 - (1 - a)^d`, which rises with depth. Requiring more
judges can reduce false acceptance under other assumptions, but it necessarily
changes the precision/coverage trade-off. Strictness is not free assurance.

### C2 — correlated majority adds no channel

Three apparent judges are aliases of one source. A shared bit `Z` is correct
with probability `0.6`; every judge emits `Z`. Majority accuracy remains `0.6`.
An independence calculation counts one information channel three times.

### C3 — unanimity adds no assurance

Two wrappers share one error bit and always agree. Each has accuracy `0.8`.
Conditioning on unanimity leaves accuracy at `0.8`; unanimity is certain but
truth is not.

### C4 — independent incompetent voters become worse

Three conditionally independent judges each have accuracy `0.4`. Majority
accuracy is:

```text
3 * 0.4^2 * 0.6 + 0.4^3 = 0.352
```

Independence without a competence-direction assumption does not amplify truth.

### C5 — recursive checking has a wrong fixed point

A judge has one stable blind spot. It accepts a false answer and accepts every
critique asserting that its earlier acceptance was correct. Arbitrary recursive
depth returns the same wrong fixed point because no new observation entered.

### C6 — optional stopping changes the reported rate

A no-skill binary assessor supports a claim with probability `0.5`. Assume two
independent attempts, but stop and report after the first support. The
probability of reporting support is:

```text
1 - (1 - 0.5)^2 = 0.75
```

The source did not become more competent. A fixed-horizon guarantee does not in
general retain its nominal validity after outcome-dependent stopping; a
stopping-safe method or specifically licensed rule is required.

### C7 — post-selection defeats internal provenance

Run one supporting and one refuting attempt, then publish only the supporting
attempt. A perfect derivation graph over the published atom remains a selected
graph. Acquisition lineage and dispositions must precede the evidence graph.

### C8 — aliases manufacture a vote

Copy one support atom under three execution IDs and apply majority voting. A
naive counter reports three votes. Exact content addressing and canonical sets
reject an exact duplicate with the same atom ID. Copies with different attempt
IDs remain distinct provenance, but their number does not change the reference
presence interpreter's four-way state name or labels. Semantic aliases and
paraphrases are harder: resistance to them is an empirical or oracle-relative
test, not a generic decidable law.

### C9 — conflict is not absence

An empty graph and a graph with one support plus one refutation can both project
to `inconclusive`. Their epistemic states are different: one is `neither`, the
other `both`. A projection may merge them for compatibility but must not erase
the underlying distinction.

### C10 — best-of-N optimizes the proxy

Candidate `A` is true with proxy score `0`; candidate `B` is false with proxy
score `1`. Best-of-N selects `B` whenever one is present. Increasing search can
raise the verifier score while lowering truth quality.

### C11 — disagreement can be the target

Two legitimate perspectives support incompatible labels for a subjective
claim. Majority erases one perspective; unanimity abstains. Neither establishes
a hidden objective truth. The annotation distribution and perspective lineage
are evidence, not noise to remove by default.

### C12 — service health is not semantics

A correct checker times out during an outage. Mapping timeout to refutation
makes the target claim change when service health changes. The timeout remains
an operational fact unless a named missingness rule consumes it.

### C13 — a valid log can omit the world

An operator appends only favorable attempts to a cryptographically valid,
append-only log. Inclusion and consistency proofs establish integrity for what
was submitted; they do not establish omission-free acquisition outside the
attested channel.

### C14 — adaptive holdout search finds a perfect accident

Evaluate `1,024` classifiers against the same ten fixed binary labels, with
every classifier prediction generated by an independent fair coin, and select
the best. The probability that at least one classifier is perfect is:

```text
1 - (1 - 2^-10)^1024 ~= 0.6323
```

A perfect selected score is exploratory evidence about an adaptive session,
not fresh confirmation on an untouched holdout.

## Positive controls

The research must recognize real amplification paths as well as reject false
ones.

### P1 — checked existential witness

For claim `exists x. P(x)`, acquisition produces a new witness `w` and a named
checker whose soundness for `P` is an explicit accepted theory establishes
`P(w)`. Determinism or admission alone is insufficient. The derivation adds a
new premise and a mechanically checked rule; it is not repetition of testimony.

### P2 — calibrated sequential evidence

A declared stochastic protocol may increase assurance when it binds its target
population, sampling frame, dependence model, statistic, calibration evidence,
information history, and fixed-horizon or anytime-valid stopping semantics. The
amplification is relative to those assumptions, not to the number of calls.

## Consequences for S2

The candidate language must therefore:

1. put acquisition and selection before evidence;
2. preserve `neither` separately from `both`;
3. keep operational facts disjoint from claim evidence;
4. use exact content identity, reject duplicates, and defer semantic alias
   equivalence to a separately revisioned profile;
5. make trust, assumptions, interpreter, projection, and resources part of the
   replay basis;
6. provide no default probability, voting, or confidence scalar;
7. require an explicit rule for every modality conversion or assurance claim;
8. expose unresolved conclusions without treating them as execution failures;
9. keep the stable verifier and report meanings unchanged; and
10. remain independent of provider, process, filesystem, socket, signal, and
    platform concepts.

## S1 gate

S1 passes only when an independent review confirms that:

- every boundary claim is linked to a primary result in
  [13-evidence-research-ledger.md](13-evidence-research-ledger.md) or has a
  self-contained countermodel here;
- every core term has an observable or formal criterion;
- each naive aggregation rule has a minimal falsifying case;
- the positive controls are representable without weakening a negative law;
  and
- no term silently equates authorization, integrity, availability, consensus,
  or repetition with truth.
