# 11 — Compatibility and versioning for epistemic research

> **Status: preparation contract.** This document freezes the Selta 0.1
> behavior that the evidence-and-decision research must not silently change.
> It does not promise that an experimental evidence interface will ship.

## Baseline

The research branch starts from:

```text
commit:            368d4b9835a80b56c6685a2578823820726ef408
cargo version:     0.1.0
schema language:   selta.schema-language/1
meta-validator:    selta.meta-validator/1
host protocol:     1
Cargo.lock SHA-256: af1a7eda12855aca5092d78889f2f6e445b7a5459230e563b0866ad4a758bfef
```

The first observed test environment is `aarch64-apple-darwin` with Rust and
Cargo 1.93.0. That observation is not yet an MSRV promise. Before reference
code, the baseline manifest must also bind the tested feature/target matrix,
canonical serialization profile, builtin and registry manifest, external host
implementations, runtime configuration, and toolchain or MSRV policy. Revision
strings alone are not reproducibility evidence; content digests and clean-tree
attestation are required.

The canonical Selta checkout used by existing consumers remains at this
baseline. Research and implementation occur in an isolated Git worktree and
branch so a path dependency cannot adopt experimental source accidentally.
There is one repository and history; the worktree is an isolation mechanism,
not a second Selta authority.

## Frozen 0.1 contract

The following behavior is legacy authority until an explicitly versioned
migration says otherwise.

| Surface | Frozen contract |
|---|---|
| Schema DSL | Closed `Node` grammar; verifier leaves; `all_of`, `any_of`, `not`; `Sampling` and threshold voting |
| Strict admission | Duplicate-safe raw-source admission, stable issue codes and JSON pointers, registry/effect/input checks |
| Core call | `verify(Node, Input, env, Options, Runtime) -> Report` |
| Host declaration | Determinism, semantic revision, cacheability, effect, input domain, needs, config/settings/delta schemas |
| Host result | Protocol-1 binary envelope `{ verdict: pass | fail, delta?, usage? }` |
| Runtime semantics | K3 fold, current union behavior, sampling/quorum policies, depth-based validation of failing deltas |
| Report | Current verdict tree, negative deltas, errors, notices, vote tallies, usage, timing, and ordering |
| Builtins | Existing default declaration set and semantics |
| Catalog | Immutable numbered schema source per pool |

JSON-additive does not necessarily mean Rust-compatible: several public types
are constructed exhaustively by consumers. Required fields, enum variants,
defaults, builtin declarations, and report classifications are therefore
protected even when Serde could parse an addition.

## Current reproducibility gap

The catalog records immutable schema bytes and a schema version, but a stored
schema does not bind the schema-language and evaluation-semantics revisions
that interpret it. Changing the engine could therefore change the meaning of
an old immutable version.

Evidence work must not exploit that gap. Before any new semantics can become
authoritative, an evaluated contract must bind at least:

- schema-language revision;
- evaluation-semantics revision;
- host-protocol revision;
- report or evidence representation revision;
- acquisition-manifest and trace revisions;
- trust-base and assumption revisions where applicable;
- interpreter revision; and
- conclusion-projection revision.

Catalog migration and replay rules require their own ADR and conformance
fixtures. They are not an incidental server change.

## Compatibility strategy

### Research and reference stages

- Do not edit the stable worktree.
- Do not change `selta-core`, the schema grammar, protocol 1, or default
  builtins merely to host the experiment.
- Keep candidate evidence types and examples opt-in and independently
  versioned.
- Treat all candidate names and shapes as provisional.
- Add a CI source-attestation sentinel before any experimental code can be
  consumed through a live path. Worktree separation prevents accidental drift
  only while the canonical checkout remains unchanged.

### Shadow stage

- Existing `verify` remains authoritative.
- Experimental evidence may be derived from already recorded legacy calls, or
  generated using a separately allocated shadow quota and trace. It cannot alter
  the authoritative call order, legacy verdict, report, retry decision, budget,
  cache, or host lifecycle.
- A legacy adapter must reconstruct the existing vote and K3 result exactly
  from recorded legacy observations before the experiment can claim
  conservativity.

### Interface-candidate stage

The interface RFC must choose explicitly between:

1. a separate `assess` lane beside unchanged `verify`;
2. a capability-negotiated protocol-2 result; or
3. a separately versioned evidence-bearing report.

Mutating the protocol-1 envelope or reinterpreting existing report fields is
not an option. A separate assessment lane is the current research preference,
not a settled design.

### Migration stage

Migration policy never authorizes changing the canonical checkout, dependency
source, or pins without explicit acceptance. This matters because a live path
dependency does observe edits to its target. Migration requires:

- an explicit candidate notice;
- old/new conformance results;
- differential execution over the consumer's existing schemas and vectors;
- classified behavioral differences;
- atomic source, suite, protocol, and schema/interpreter pin updates;
- a consumer-owned acceptance decision; and
- a tested rollback to the prior baseline.

## Change classification

Every candidate change is classified before implementation.

| Class | Examples | Requirement |
|---|---|---|
| Internal research | Non-exported proof model, documents, fixtures | No legacy source or behavior change |
| Additive opt-in | New separately versioned crate, method, or negotiated capability | Old consumer and host suites unchanged |
| Behavioral | Changed default, fold, union, sampling, budget, error, or report meaning | New semantics revision and explicit migration |
| Source-identity | Any runtime source included by a consumer trust manifest | New source identity even if behavior is equal |
| Breaking | Public Rust shape, DSL grammar, protocol-1 envelope, builtin inventory | Major interface decision and adapter or rejection |

An interface notice is required for public API, grammar, builtin inventory,
existing schema behavior, typed error, verdict/report, cache/sampling,
deadline, or resource-bound changes.

## Legacy differential corpus

Before production integration, the baseline must be characterized by golden or
semantic-equivalence fixtures covering:

- schema serialization and strict-admission issues;
- protocol-1 initialization, manifests, calls, envelopes, and errors;
- pass, fail, and inconclusive reports;
- `all_of`, `any_of`, `not`, and union selection;
- deterministic and sampled leaves;
- quorum failure, malformed samples, timeouts, and budget exhaustion;
- depth-zero skipping and nested delta-schema validation;
- cache identity and `NoCache` execution;
- builtin declaration inventory and config semantics; and
- catalog replay under the pinned 0.1 interpreter.

The adapter may need the legacy execution trace—not merely successful leaf
observations—to reproduce fail-fast choices, traversal and report ordering,
skipped nodes, cache outcomes, budgets, errors, timing classes, and complete
reports. The fixture manifest must classify each field as byte-identical,
semantically identical, intentionally non-deterministic, or excluded with a
reason. Equality of the root K3 verdict alone is insufficient.

The compatibility target is byte identity where the existing contract promises
it and semantic identity otherwise. The distinction must be listed per fixture;
it cannot be decided after a difference appears.

## Host compatibility matrix

The future interface RFC must test all four combinations.

| Consumer | Host | Required result |
|---|---|---|
| Legacy | Protocol 1 | Exact existing behavior |
| New | Protocol 1 | Legacy adapter or explicit unsupported capability; never reinterpretation |
| Legacy | Protocol 2 | Host negotiates protocol 1 or is rejected before calls |
| New | Protocol 2 | Evidence behavior under pinned revisions |

Capability negotiation must precede registration. A host cannot self-declare a
protected proof status, source independence, or trust level; those authorities
belong to admitted policy and rule records.

## Compatibility invariants

1. Research policy and CI source attestation prevent a stable consumer from
   observing the research branch without opting in; a path dependency alone
   provides no such protection.
2. A stored 0.1 schema never acquires new semantics from a newer executable.
3. Operational errors never become semantic evidence in the legacy adapter.
4. A legacy clean pass remains a clean pass with no newly populated error
   channel.
5. Existing union, combinator, vote, and depth behavior is unchanged on the
   legacy path.
6. Default builtin inventory is unchanged without a breaking notice.
7. A new evidence interface cannot reuse an old revision identifier.
8. Rollback restores both code and the complete interpretation basis.

## Promotion gate

The research may request consumer evaluation only after:

- the theory and DSL have survived their falsification suite;
- the stable Selta suite still passes unchanged;
- the legacy differential corpus is complete;
- protocol and report compatibility matrices pass;
- evidence generation is resource-bounded;
- acquisition traces prevent silent post-selection and adaptive-stopping
  omissions;
- canonical encoding, domain-separated hashes, downgrade handling, privacy
  leakage, forgery, and adversarial resource cases have passed review;
- source compatibility, Serde compatibility, feature/target matrix, and MSRV
  checks pass;
- trust expiry, revocation, supersession, historical replay, and current
  re-evaluation have distinct semantics;
- real-model and non-model case studies show a measurable advantage at matched
  cost; and
- a release-candidate notice lists exact symbols, DSL constructs, revisions,
  source changes, known differences, migration adapter, and rollback.

Until then the only correct dependent-interface notice is:

```text
status: research only
supported baseline: 368d4b9835a80b56c6685a2578823820726ef408
consumer action: none
migration trigger: explicit release-candidate notice
```
