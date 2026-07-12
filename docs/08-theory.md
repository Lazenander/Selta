# 08 — Theoretic model

Selta's legacy path is a proof-relevant, three-valued refinement type system over the
algebra of JSON values, where failure evidence lives in a change structure,
non-deterministic predicates may be modeled as Markov kernels decided by threshold
voting, the judge-the-judges circularity is broken by step-indexing on depth, and the
engine is a handler for verifier effects. The revision-2 cautious path adds a
four-valued semantic layer and projects it back to the same report verdicts. This
document unpacks those models; probabilistic statements apply only under the explicit
assumptions stated below.

## Values — the initial algebra of JSON

```text
V = μX. 1 + Bool + Num + Str + List X + Map Str X
```

The value universe is the initial algebra of the JSON signature functor. Totality of the
engine's traversal is structural recursion on this algebra, and a path (`$.code`,
`$.tests[2]`) is simply a position in an inductive term.

## Schemas — refinement types

A schema denotes a subset of the universe:

```text
⟦S⟧ = { v : V | φ_S(v) }  ⊆  V
```

The structural part carves the underlying type; verifiers refine it with semantic
predicates. Registration-time meta-validation is kind checking. The refinements form a
bounded lattice: `fields` and `all_of` are meets, `union` and `any_of` are joins, `any`
is ⊤ — type-level "or" and check-level "or" are the same join at two levels.

## Verdicts — Kleene's strong three-valued logic

Verdicts live in K3 with the order `fail < inconclusive < pass`:

| Operator | K3 | Where it appears |
|---|---|---|
| `all_of`, field folds, the report fold | min (∧) | [03-verification.md](03-verification.md) fold rule |
| `any_of`, union | max (∨) | combinator table |
| `not` | swaps extremes, fixes unknown | combinator table |

Associativity, commutativity, and De Morgan duality hold, so the report's verdict — one
big K3 conjunction over the tree — is independent of evaluation order. These laws are
property tests in `selta-core`. On the legacy path, "errors never vote" preserves K3
unknown as operational absence of a conclusion. The cautious projection also uses K3
unknown for semantic conflict; its evidence summary keeps those meanings distinct.

An opted-in cautious subtree first uses the four presence states `(support, refute)`:
`neither`, `support_only`, `refute_only`, and `both`. Repetition takes knowledge join;
`not` swaps polarity; `all_of` and `any_of` use the truth operations specified in
[24-presence-assessment.md](24-presence-assessment.md). Only after composition does Selta
project `support_only` to pass, `refute_only` to fail, and conflict or absence to
inconclusive. This preserves distinctions that K3 alone cannot represent without
changing the legacy report algebra.

## Deltas — change structures

Fix a change structure `(V, Δ, ⊕)`: for each value `v`, a set `Δ(v)` of changes and an
update `v ⊕ δ`. Verification is a proof-relevant characteristic function:

```text
χ_S(v)  ∈  {pass}  ∪  {fail} × Δ(v)  ∪  {inconclusive} × E
```

A failing witness `δ` is a *direction* from the current answer toward an acceptable one:
`v ⊕ δ` is intended to land in (or toward) `⟦S⟧`. Two design facts are theorems of
scope: Selta computes `δ` but never applies `⊕` (the report boundary, stated
mathematically), and union best-match selects the witness minimal in the closeness
preorder on `Δ(v)` — verification quietly estimates a distance to acceptability, and
pass means distance zero.

## Sampling and voting — a conditional kernel model

A non-deterministic verifier can be modeled as a kernel
`k : V ⇝ Verdict × Δ` when its environment is stable enough to define such a
distribution. Under that model, threshold voting estimates "P(pass | valid) ≥ θ", with
θ = ½ for `majority`, θ = f for `ratio(f)`, and the obvious counts for `at_least` and
`unanimous`. Three engine rules have a probabilistic reading:

- verifying each sample's delta against `delta_schema` is **conditioning** the kernel on
  well-typed evidence;
- resampling malformed replies is **rejection sampling** from that conditional;
- `min_valid` guards against conditioning on a near-null event.

Convergence is conditional, not an engine guarantee. If valid executions are independent
and identically distributed from one stationary kernel, rejection does not introduce an
unmodeled selection bias, and the conditional pass probability is separated from the
threshold, then the empirical frequency converges almost surely to that probability.
Selta does not establish those premises: separate calls can be correlated, drift over
time, or share upstream state. Even under the premises, convergence is to the configured
verifier's behavior, not to objective correctness. A deterministic verifier is the
declared degenerate case — a Dirac kernel — which licenses run-once execution; caching
additionally requires the separate purity and cacheability claims in
[05-extensions.md](05-extensions.md).

## Depth — step-indexing

Deltas are typed; delta schemas carry verifiers; those verifiers emit typed deltas. The
circularity is broken by step-indexing, with depth as the index:

```text
⟦S⟧₀     — deterministic fragment only; non-deterministic checks are skipped
⟦S⟧_d+1  — kernels allowed; their witnesses are validated in ⟦delta_schema⟧_d
```

Termination is well-founded induction on `(depth, schema size)`, lexicographically. This
is the same maneuver step-indexed logical relations use to break the circularity of
recursive types, doing here exactly what it was invented for.

## Engine — a handler

A verification is a program in the free monad over the signature of verifier operations,
graded by depth; the engine is a handler. Determinism is an equation on operations —
`verify; verify ≡ verify` with equal results — which holds precisely for Dirac kernels,
so the cache is a sound handler transformation on deterministic operations and nowhere
else. The scripted fake extension used in tests, the real host over JSON-RPC, and the
cache are three handlers for one program; swapping them preserves semantics by
construction.

## What the model buys, concretely

| Law | Consequence in code |
|---|---|
| K3 algebra laws | fold order never matters; property tests in `selta-core` |
| Dirac ⇒ copyable | cache only ever consulted for deterministic extensions |
| Conditioning + quorum | malformed votes rejected and resampled; `min_valid` ⇒ `inconclusive`, not `fail` |
| Step-index well-foundedness | recursion on `(depth, node)` provably terminates |
| Handlers | test hosts, RPC hosts, and cache are interchangeable behind one trait |
