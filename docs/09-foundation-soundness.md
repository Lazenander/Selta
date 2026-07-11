# 09 — Foundation soundness hardening

Status: accepted design. H0 runtime soundness is implemented and locked; H1
strict admission and H2 Cosmiz cutover remain staged. This document records invariants that
must hold together and the tests that make each claim executable.

Selta v0.1 is an executable prototype, not yet a sufficient sole admission
authority for Cosmiz. The existing API assumes callers meta-validate a parsed
`Node`, while the public verifier must nevertheless remain fail-closed for any
parseable value. The schema parser also accepts shapes that Serde can flatten
by discarding unknown fields. Cosmiz currently compensates with a duplicate
strict raw walker. The following slices close those gaps without conflating
Selta's general union semantics with Cosmiz's domain-specific exact-one rule.

## H0 — Runtime soundness

### Speculative control flow

Union variants are speculative. A rejected variant may use a private fail-fast
flag, but it must not stop or skip a later variant. Only the final union result
may set the request's fail-fast flag. An empty union is a structural failure,
never a panic. A primitive type mismatch must prevent its attached verifiers
from running, including a non-integral number at an `int` node.

Delta evidence is accepted only when its declared schema returns `pass`.
`inconclusive` means Selta could not establish that the evidence is well typed;
it must therefore reject the host result rather than turn it into a user-facing
failure.

Sampling policies require at least one valid vote. `min_valid: 0`,
`at_least: 0`, invalid ratios, and a zero-valid-vote unanimous result are never
successful, even if a caller bypasses catalog meta-validation.

### Cache identity and eligibility

Determinism controls run-once versus sampling; it does not by itself imply
referential transparency. Each extension declaration therefore owns:

- a non-empty `semantic_revision` that identifies the verifier semantics; and
- an explicit `cacheable` bit.

External declarations default to non-cacheable. Pure in-process builtins may
opt in; `cmd` is process I/O and is always non-cacheable. A cache key binds the
extension name and semantic revision, resolved config, settings fingerprint,
value, JSON path, recursion depth, and exactly the root/environment context the
declaration requests. Two registries may share a cache only when equal semantic
revisions honestly denote equal behavior. Lying about revision, determinism, or
cacheability is an extension trust-boundary violation.

### Host containment and accounting

A timed-out `cmd` child is killed on future drop; it may not continue changing
the machine after Selta reports an error. Host usage is untrusted. Token sums
use checked arithmetic, cost must be finite and non-negative, and an invalid or
overflowing usage envelope becomes an execution error rather than a panic or a
wrapped accounting value.

H0 acceptance includes deterministic regressions for:

- fail-fast union isolation and empty-union failure;
- cache separation by path, depth, and semantic revision;
- non-cacheable `cmd` behavior and timeout child termination;
- pass-only delta-schema evidence;
- usage overflow/invalid-cost rejection;
- primitive mismatch suppression; and
- positive quorum/vote-policy requirements; and
- atomic rejection of duplicate names inside one registry batch.

## H1 — Additive strict admission

`Node::from_value` remains the compatibility parser during v0.1. A new additive
strict API admits raw JSON into an opaque `AdmittedNode` and returns typed,
pointer-addressed `MetaIssue`s. Catalog/server and proof-oriented consumers use
that API; callers that deliberately use the compatibility parser receive no
admission claim.

Strict admission owns:

- recursive node-key closure and type-specific required fields;
- `required` only at object-field positions;
- exactly one verifier discriminant and closed combinator/leaf/sampling shapes;
- non-empty unions/combinators and ordered array bounds;
- well-formed exact `{ "$env": "dot.path" }` holes plus validation of every
  literal sibling around holes;
- extension existence, effect/input-domain compatibility, sampling policy, and
  config structure; and
- Selta-owned semantic preflight for literal builtin configs, repeated after
  dynamic config resolution.

Extension declarations add a semantic revision, effect class, cacheability,
and accepted input domain. Selta exposes a pure builtin registry/profile that
excludes `cmd`; `with_builtins` remains the compatibility constructor. Core
builtin correctness rejects empty `one_of`, empty/inverted `range` and `len`,
negative length bounds, and invalid regex. Consumer resource ceilings such as
Cosmiz's 4,096-entry and 4,096-byte limits remain explicit admission-policy
inputs rather than hidden application logic in Selta.

The schema-language and meta-validator revisions are exported constants. The
strict issue vocabulary and JSON-pointer locations are stable protocol data,
not prose parsed from `Vec<String>`.

## H2 — Cosmiz cutover

Cosmiz moves its canonical schema loader to Selta's strict admission API and
pure registry, then deletes duplicate node/verifier/config shape logic.
Cosmiz retains only its own proof and policy:

- duplicate-safe I-JSON and canonical bytes;
- schema IDs, self fields, schema/value digests, and exact bootstrap pins;
- Foundation resource ceilings and the exact admitted extension manifest;
- nested dynamic-carrier closure and exact-one union selection; and
- source/conformance/build attestation boundaries.

Every Selta source change regenerates Cosmiz's Selta source closure,
conformance manifest, aggregate bootstrap, and Rust pins. The cutover is atomic:
both repositories' locked suites and Cosmiz's bootstrap generator check must be
green before the duplicate loader is removed.

## Explicit non-claims

- H0 does not make an unversioned external extension cache-safe.
- H1 does not make `Node::from_value` strict or remove it in v0.1.
- Selta's general union remains first-pass/best-failure; Cosmiz exact-one dynamic
  union admission is a separate domain proof.
- A pure registry proves only declared in-process effect classification, not
  full build or executable attestation.

## H0 verification record

At the H0 checkpoint:

```text
cargo test -p selta-core
43 passed; 0 failed

cargo clippy -p selta-core -p selta-protocol -p seltad --tests -- -D warnings
passed

cargo test -p seltad declaration_json_exposes_cache_identity_and_eligibility
1 passed; 0 failed

node --check packages/extension/index.js
node --check crates/selta-core/tests/fixtures/host.mjs
passed
```

The 43 core tests include nine runtime-soundness regressions and six cache-
identity regressions. Live host tests for the TypeScript and Lean adapters also
passed in this environment.
