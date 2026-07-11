# 03 — Verification semantics

This document defines what the engine does with `(schema, value, env)`. The output format
it produces is specified in [04-report.md](04-report.md).

## Pipeline

```text
raw input ─► intake ─► structural check ─► node verifiers ─► recurse into children ─► report
```

Stages 2–4 run per node, recursively over the schema tree. The engine collects everything
by default; it does not stop at the first failure (a `fail_fast` request option exists for
callers who want cheap early exits).

### 1. Intake

Applies only when the caller submits raw text rather than parsed JSON. In lenient mode
(default) intake strips markdown fences and repairs minor JSON damage, recording every
repair as a notice. In strict mode any repair that would have been needed is a `structure`
delta instead. Intake never invents content: if the text is unrecoverable, verification
fails structurally at `$` and stops.

### 2. Structural check

The node's type is checked against the value per the semantics in
[02-schema.md](02-schema.md): type match, required fields, closed-object keys, array
length, union best-match. Every violation is a `structure` delta. Structural checking is
deterministic and built into the engine — it is the one class of check that is not an
extension, because everything else depends on it.

If the value's shape is wrong at this node, attached verifiers do not run (there is
nothing meaningful to hand them), but sibling subtrees are still verified.

### 3. Verifiers

Each `VerifierSpec` on the node runs according to the extension's declared determinism.
Before dispatch, the engine resolves `$env` references in the spec's config
([02-schema.md](02-schema.md)) and validates the resolved config against the extension's
`config_schema`; a missing reference or a failing resolved config makes the check an
error (`inconclusive`) — the request is malformed, not the value.

**Deterministic** — executed once. `pass`, `fail(delta)`, or `error` (timeout, crash,
missing tool). A failing check's delta is a typed value and is verified against the
extension's `delta_schema` at `depth − 1`; only a `pass` is accepted. A `fail` or
`inconclusive` delta-schema result turns the check into an error, never a delta against
the user's value. Eligible results may be cached by content hash (below).

**Non-deterministic** — executed as a sampling round:

1. The engine requests `samples` independent executions from the host. It materializes
   them in windows of at most `MAX_SAMPLE_IN_FLIGHT_PER_JOB` (currently 64); this bounds
   per-job futures and host-call concurrency without changing the requested vote count,
   retry allowance, or shared request budget.
2. Each execution returns a result envelope or an error. The envelope is verified before
   it may count as a vote:
   - always: structurally, against the built-in vote schema
     (`{ "verdict": "pass" | "fail", "delta"?: Delta }`);
   - additionally, for a failing vote: the delta is itself a typed value, and it is
     verified against the extension's declared `delta_schema` with budget `depth − 1`.
     This is the recursive step — a judge's delta is itself LLM output and gets the
     same treatment as any other value, including any verifiers the `delta_schema`
     carries (non-deterministic ones re-enter this pipeline one level down).
3. An envelope that fails its verification is an **error, not a vote**. The engine
   resamples while the sample budget allows; otherwise the slot stays an error.
4. Votes are folded by the policy (next section).

**At `depth == 0`** non-deterministic verifiers do not run at all: any such check is
skipped and recorded as a notice (`skipped: depth exhausted`). A skipped check contributes
`inconclusive` pressure only if it was the node's only check; it is never a silent pass.
Since every recursion step decreases `depth` by one and `depth` is finite, verification
terminates.

### 4. Recursion into children

Object fields and array elements are verified with the same pipeline, with the path
extended (`$.code`, `$.tests[2]`). Child results fold into the parent (below).

## Verdicts

Three values, because errors are not differences:

| Verdict | Meaning | Carries |
|---|---|---|
| `pass` | The value is acceptable at this node | — |
| `fail` | The value differs from acceptable | one or more deltas |
| `inconclusive` | Selta could not determine the answer | error details |

A verifier crash, a timeout, an unreachable model API — none of these mean the value is
wrong, and none of them may ever be reported as a delta or counted as a failing vote.
Callers can distinguish "regenerate the answer" (`fail`) from "re-run the verification"
(`inconclusive`); what they do about it is out of scope.

**Fold rule** (node verdict from its own checks and its children):

```text
fail        if the structural check failed, any verifier failed, or any child failed
inconclusive  otherwise, if any verifier or child was inconclusive
pass        otherwise
```

The report's top-level verdict is the fold at `$`.

## Voting

Only valid votes participate; errors reduce the vote count.

```text
valid = samples that produced a verified envelope; at least one is always required
if |valid| < min_valid          → check is inconclusive (quorum failure)
else apply policy over valid:
  majority        pass iff pass_votes > fail_votes
  unanimous       pass iff fail_votes == 0
  at_least(k)     pass iff pass_votes ≥ k
  ratio(f)        pass iff pass_votes / |valid| ≥ f
```

Defaults: `samples = 3`, `vote = majority`, `min_valid = ceil(samples / 2)`, `depth = 1`.
`samples`, `min_valid`, and `at_least(k)` must be positive; ratios must be in `(0, 1]`.
The runtime applies the same fail-closed predicates if an embedded caller bypasses
meta-validation, and it caps futures allocation before using an untrusted sample count.

On a failing outcome, the deltas of the failing votes are merged into one delta for the
check: distinct messages are kept (they are the difference, in words), byte-identical
messages are deduplicated, and the vote tally (`pass / fail / errors / samples`) is
attached as metadata.

## Verifier composition

A node's `verify` array is an implicit `all_of`. Beyond that, specs compose
(grammar in [02-schema.md](02-schema.md)):

| Combinator | Verdict |
|---|---|
| `all_of` | `fail` if any child fails (all failing deltas kept); else `inconclusive` if any child is; else `pass` |
| `any_of` | `pass` if any child passes; else `inconclusive` if any child is (a pass might still have been possible); else `fail` with the deltas of the fewest-delta child |
| `not` | inverts `pass`/`fail`; `inconclusive` stays; a failing `not` reports the `message` from its spec, since the passing inner check produced no delta to negate |

`any_of` mirrors union best-match one layer up: `union` is "or" over types, `any_of` is
"or" over checks. Children are evaluated in document order and `any_of` stops at the
first pass. A combinator's determinism is derived — non-deterministic iff any child
is — and `sampling` attaches only to leaves, so voting always happens at the level of a
single extension.

## Budgets

A budget rides in the context and only ever shrinks:

| Field | Meaning |
|---|---|
| `depth` | Remaining recursion for non-deterministic result verification |
| `samples` | Remaining sample executions for this request (all checks combined) |
| `deadline` | Wall-clock cutoff for the whole request |

Pool-level caps (`max_depth`, `max_samples`, concurrency, spend — see
[06-server.md](06-server.md)) bound execution. At catalog admission, every
non-deterministic leaf's explicit `sampling` request — or Selta's default when it is
omitted — must individually fit the pool's depth and sample caps. This makes one leaf
invocation feasible; it does not reserve that budget. The request-wide counter still
arbitrates across sibling checks, repeated array items, and retries. When that shared
budget runs out mid-verification, affected checks become `inconclusive` with an
explanatory error — never `fail`.

## Caching

Verifier executions are memoizable by content hash:

```text
key = hash(extension name, semantic revision, config, resolved settings fingerprint,
           value, JSON path, recursion depth,
           root/env fields the extension declared in `needs`)
```

Only deterministic extensions that explicitly declare themselves `cacheable` use the
cache. Determinism controls execution strategy; it does not prove referential transparency.
Settings, path, and depth participate because a verifier may observe all three, while the
semantic revision prevents results surviving a behavior change under the same extension
name ([05-extensions.md](05-extensions.md)). External extensions default to non-cacheable.
Non-deterministic vote sets are reused only within a single request (a retry loop
re-judging unchanged fields is the caller's concern, not the engine's). The cache remains
a trait and `NoCache` is the explicit no-op implementation. `MemoryCache::default()` is a
deterministic least-recently-used cache bounded to 16,384 entries and 128 MiB of
approximate serialized-envelope weight. `MemoryCache::with_limits` selects smaller or
larger bounds; either zero limit disables caching. A durable store is a server concern.

## Cancellation and timeouts

Every in-flight verifier execution carries the request deadline. Cancelling a request
propagates `cancel` to hosts for all its in-flight executions ([05-extensions.md](05-extensions.md)).
A verifier that outlives its per-call timeout is treated as an error (→ `inconclusive`
pressure), and the host is considered unhealthy after repeated timeouts — supervision is
the server's job. The `cmd` builtin additionally kills its child when a timeout cancels
the wait future. The engine guarantees: no hung extension can hang a verification past
its deadline.
