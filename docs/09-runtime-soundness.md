# 09 — Runtime soundness hardening

Status: accepted design. H0 runtime soundness and H1 strict admission are
implemented and locked; H2 consumer migrations remain integration work. This
document records invariants that must hold together and the tests that make
each claim executable.

Selta v0.1 began as an executable prototype whose compatibility API assumes
callers meta-validate an already parsed `Node`, while the public verifier must
nevertheless remain fail-closed for any parseable value. The compatibility
schema parser also accepts shapes that Serde can flatten by discarding unknown
fields. Proof-grade consumers therefore needed an independent raw-shape gate.
The following slices close those gaps without conflating Selta's general union
semantics with a consumer's stricter domain-specific selection rules.

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
negative length bounds, and invalid regex. Consumer resource ceilings remain
explicit admission-policy inputs rather than hidden application logic in
Selta.

The schema-language and meta-validator revisions are exported constants. The
strict issue vocabulary and JSON-pointer locations are stable protocol data,
not prose parsed from `Vec<String>`.

The additive core seam is raw-source only:

- `AdmittedNode::admit_source(&[u8])` rejects duplicate keys before a JSON map
  can overwrite them and constructs the opaque proof only after closed grammar
  projection succeeds;
- `AdmittedNode::admit_source_with_config_validator` lets a registry add typed
  config/extension issues without creating a second grammar path;
- `Registry::admit_source(source, policy)` composes that raw proof with the
  concrete registry: extension existence, effect grant, input domain,
  deterministic sampling, env-hole-aware config structure, literal semantic
  preflight, and optional generic builtin resource ceilings;
- `AdmissionPolicy::new` enumerates granted `EffectClass` values;
  `AdmissionPolicy::pure_only` is the least-effect builtin profile, while
  `BuiltinAdmissionLimits` optionally bounds `one_of` values and regex UTF-8
  bytes without naming or embedding a downstream product;
- `validate_config_structure_with_env_holes` treats only an exact, well-formed
  `{ "$env": "dot.path" }` object as deferred and continues checking every
  literal sibling; and
- `SELTA_SCHEMA_LANGUAGE_REVISION`, `SELTA_META_VALIDATOR_REVISION`,
  `MetaIssueCode`, and RFC 6901 `MetaIssue.pointer` are the integration contract.

There is deliberately no admitted `serde_json::Value` constructor: once a
duplicate-tolerant parser has produced a map, uniqueness evidence is
irrecoverable. `Node::from_value` remains available but carries no admission
claim.

Registry batches are prospective and atomic. Before the registry mutates,
every declaration schema is serialized and re-admitted through the strict
grammar; config and settings schemas reject verifier annotations recursively;
and every delta schema is meta-validated against a temporary registry that
already contains the complete incoming batch. This permits intentional
cross-declaration delta references but never leaves a partial batch behind.

## H2 — Consumer migration

A proof-grade consumer moves its canonical schema loader to Selta's strict
raw-source admission API and an explicit registry/policy, then deletes any
duplicate general node, verifier, and builtin-config grammar. The consumer
retains only its domain proof and policy, which may include:

- canonical byte and digest rules;
- application schema identities and self-binding fields;
- resource ceilings and an exact admitted extension profile;
- nested carrier closure or stricter union-selection proofs; and
- source, conformance, and build-attestation boundaries.

Consumers that pin Selta source or conformance identity regenerate those pins
after every accepted Selta source change. A migration is atomic: Selta's locked
suite, the consumer's suite, and any generated-artifact consistency checks must
be green before its duplicate loader is removed.

## Explicit non-claims

- H0 does not make an unversioned external extension cache-safe.
- H1 does not make `Node::from_value` strict or remove it in v0.1.
- Selta's general union remains first-pass/best-failure; a consumer's exact-one
  admission rule is a separate domain proof.
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

## H1 verification record

At the H1 checkpoint, including the resource-bounding regressions found during
the integration audit:

```text
cargo test --workspace
selta-core: 81 passed; 0 failed
seltad: 13 passed; 0 failed
workspace/doc tests: all passed

cargo clippy --workspace --all-targets -- -D warnings
passed

node --check packages/extension/index.js
node --check crates/selta-core/tests/fixtures/host.mjs
passed
```

The core total includes 13 closed raw-grammar tests, seven registry-policy and
atomic-declaration tests, six raw RPC/cancellation tests, five builtin-profile
tests, six bounded-cache unit tests, and the large sampling-window regression.
The daemon total includes four raw catalog/HTTP and live WebSocket/server tests
plus four storage-contract tests. The live socket tests passed with localhost
binding enabled; they are not merely compile-only checks.
