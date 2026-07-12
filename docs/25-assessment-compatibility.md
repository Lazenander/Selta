# Assessment compatibility and migration

Status: implemented Selta 0.2 core/host compatibility candidate. Promotion
gates remain pending; daemon/catalog promotion remains outside this slice.

This document specifies how the presence-assessment feature in
[24-presence-assessment.md](24-presence-assessment.md) is added without
silently changing Selta 0.1. Document 24 owns the assessment semantics. This
document owns version selection, legacy embedding, RPC and SDK compatibility,
promotion, and rollback.

Normative terms **MUST**, **MUST NOT**, **SHOULD**, and **MAY** have their usual
RFC 2119 meanings.

## 1. Compatibility boundary

The following Selta 0.1 surfaces remain authoritative and unchanged:

- `verify(Node, Input, env, Options, Runtime) -> Report`;
- the behavior of every schema leaf that omits `evidence`;
- `Envelope { verdict: PassFail, delta?, usage? }`;
- `PassFail::{Pass, Fail}`;
- the JSON-RPC `verify` method and `VerifyParams`;
- legacy sampling, quorum, `VotePolicy`, K3 folding, delta validation, cache
  behavior, report ordering, and error behavior; and
- the protocol-1 initialize result and extension-manifest shapes.

The only schema opt-in is the leaf field:

```json
{ "ext": "semantic_judge", "evidence": "cautious" }
```

Absence of this field selects the complete legacy path. Presence selects the
assessment path in document 24. An implementation MUST branch on this field
before host dispatch and MUST NOT infer assessment mode from extension name,
determinism, settings, host implementation, or response shape.

The public `verify` function remains the single executor. Assessment is another
leaf semantics inside that executor, not a second traversal or a later replay.
One leaf execution calls exactly one of `ExtensionHost::verify` or
`ExtensionHost::assess`, subject only to the RPC fallback in section 4.

## 2. Explicit admission revisions

Selta 0.1 identifiers remain permanently bound to the old grammar and
validator:

```text
selta.schema-language/1
selta.meta-validator/1
```

Presence assessment introduces:

```text
selta.schema-language/2
selta.meta-validator/2
```

Revision 1 MUST reject `evidence` as an unexpected leaf property. Revision 2
MUST admit exactly the optional string value `"cautious"` and MUST apply the
additional static rules in document 24, including rejection of an authored
`sampling.vote` on an evidence leaf and rejection of a mixed legacy/evidence
combinator subtree.

Revision selection is an admission input, not a guess from document contents.
The additive API shape is conceptually:

```rust
pub enum AdmissionProfile {
    Selta1,
    Selta2,
}

AdmittedNode::admit_source_at(source, AdmissionProfile::Selta2)
Registry::admit_source_at(source, policy, AdmissionProfile::Selta2)
```

The existing `AdmittedNode::admit_source` and `Registry::admit_source` methods
MUST remain aliases for the revision-1 profile. Existing unsuffixed revision
constants MUST retain their revision-1 values. New suffixed constants expose
both supported schema and meta-validator revisions; no constant may silently
change its referent.

An `AdmittedNode` produced under revision 2 MUST retain its admission profile so
that a caller, catalog, report adapter, or audit record can recover the exact
grammar and validator pair. Only the two pairs listed above are supported; a
caller cannot freely combine schema-language revision 1 with meta-validator
revision 2 or vice versa.

If a catalog stores revision-2 schemas, it MUST store the schema-language and
meta-validator identifiers with each immutable schema version. Existing rows
and files are interpreted as the revision-1 pair without rewriting their source
bytes. A server MUST NOT auto-promote a stored revision-1 schema because a new
binary happens to understand revision 2.

Revision 2 without an `evidence` field has the same execution semantics as
revision 1. This permits mechanical migration, but it does not erase the
distinct admission identity.

## 3. In-process host compatibility

`ExtensionHost` gains one method with a complete default implementation:

```rust
async fn assess(
    &self,
    call: HostCall<'_>,
) -> Result<AssessmentEnvelope, String> {
    legacy_embed(self.verify(call).await)
}
```

The assessment result is a separate type:

```rust
pub struct AssessmentEnvelope {
    pub support: bool,
    pub refute: Option<WireDelta>,
    pub usage: Option<WireUsage>,
}
```

It does not add a variant or field to `Envelope` or `PassFail`. Its exact states
are:

| `support` | `refute` | State |
|---|---|---|
| `false` | absent | `neither` |
| `true` | absent | `support_only` |
| `false` | present | `refute_only` |
| `true` | present | `both` |

The default legacy embedding is exact:

| Legacy result | Assessment result |
|---|---|
| `Pass` | `support: true`, no refutation, usage preserved |
| `Fail` with a valid delta | `support: false`, refutation present, usage preserved |
| `Fail` without a delta | operational error |
| host error | the same operational error |

Unexpected data on a legacy pass does not become refutation. Legacy delta and
usage validation still run at their existing boundaries. The default method is
therefore source-compatible for ordinary trait implementations after rebuild:
an existing host need not implement `assess`, and an evidence leaf can still
use it through the declared legacy embedding.

A native implementation MAY override `assess` and return all four states. Its
existing extension `semantic_revision` binds both its verify and assessment
behavior; changing either behavior requires a new semantic revision. This avoids
a second declaration and a second registry.

Assessment envelopes MUST NOT enter the legacy `Envelope` cache. Selta 0.2
therefore performs no assessment caching, as required by document 24. A future
cache must use a distinct value type and operation-tagged key and requires a
separate compatibility notice.

## 4. Optional JSON-RPC `assess`

Protocol 1 keeps `initialize`, `verify`, `cancel`, and `shutdown` unchanged. It
adds one optional JSON-RPC method without changing either the initialize result
or extension manifest:

```text
method: assess
params: existing VerifyParams
result: AssessmentEnvelope
```

The same resolved config, settings, value, path, permitted root/environment,
depth, and deadline are sent as for `verify`. The returned object is a closed
wire shape: `support` is a required Boolean, `refute` is an optional
`WireDelta`, and `usage` is optional. A present refutation passes the same
delta-schema validation at `depth - 1` as a legacy failure.

`RpcHost::assess` follows this algorithm:

1. Send one `assess` request.
2. If the result deserializes as the closed `AssessmentEnvelope` shape, return it
   to the engine.
3. If and only if the peer returns JSON-RPC error code `-32601`, send one
   `verify` request and apply the default legacy embedding.
4. For every other JSON-RPC error, transport failure, timeout, cancellation,
   disconnect, or malformed response, return an operational error and do not
   call `verify`.

`assess_over_peer` owns transport, closed-envelope deserialization, and the
structured fallback decision. After it returns, the engine validates usage,
requires a non-empty refutation message, and verifies the refutation against
the extension's `delta_schema`. A failure at any of those later boundaries is
operational and MUST NOT trigger legacy fallback.

The implementation MUST retain the structured JSON-RPC error code internally.
It MUST NOT authorize fallback by matching error prose. The rule is deliberately
narrow: retrying a paid or effectful verifier after an ambiguous failure can
duplicate work and conceal a real assessor fault. A method-not-found response
establishes that no assessment handler ran; the fallback is then safe.

A peer that never answers an unknown method times out and does not fall back.
The bundled TypeScript SDK therefore must first ship correct `-32601` replies
for unsupported methods. Pre-upgrade hosts that silently ignore `assess` remain
fully compatible for legacy schemas, but an opted-in evidence leaf becomes
operationally inconclusive until that host is upgraded or implements the JSON-RPC
method-not-found rule.

The RPC extension is optional rather than negotiated in `initialize`, so the
closed initialize and extension-declaration shapes gain no assessor capability
field. Ordinary host identity/version values may change with an SDK release.
Support is discovered only by an actual evidence-mode call, and the sole
negative capability result is structured `-32601`.

## 5. TypeScript SDK assessor lane

The existing SDK exports and behavior remain unchanged:

```js
host.verifier(name, options, verifyHandler)
pass()
fail(delta)
host.run(info)
```

The SDK adds a parallel handler registration that reuses an existing verifier
declaration:

```js
host.assessor(name, assessHandler)
support()
refute(delta)
both(delta)
neither()
```

`host.assessor` MUST require the same name to be registered with
`host.verifier`; it adds no initialize-manifest entry and owns no duplicate
config, settings, effect, input-domain, needs, determinism, or semantic-revision
fields. This keeps one extension identity and one declaration.

The helpers produce:

```js
support()     // { support: true }
refute(delta) // { support: false, refute: normalizeDelta(delta) }
both(delta)   // { support: true, refute: normalizeDelta(delta) }
neither()     // { support: false }
```

The helpers accept only the arguments shown. A handler that reports usage returns the
same object with an additive `usage` field, for example
`{ ...support(), usage }`. The TypeScript `AssessmentResult` type exactly mirrors
`AssessmentEnvelope`.

On `assess`, the SDK dispatches the named assessor. If the extension has no
native assessor, it replies with `-32601`, permitting the server's legacy
fallback. A thrown assessor exception replies with `-32000`; it MUST NOT be
reported as method-not-found. Any unknown JSON-RPC method also receives
`-32601` instead of being silently ignored.

An old server never sends `assess`; a new SDK connected to it emits the same
extension declaration shapes and values, with no assessor capability, and
serves `verify` exactly as before. Its ordinary host version may identify the
new SDK release. An old SDK continues serving legacy schemas to a new server
because evidence mode is selected only by a revision-2 leaf.

## 6. Report and Rust-source compatibility

Evidence-mode checks add the optional `CheckResult.evidence` summary specified
in document 24. It is omitted when `evidence` is absent. Consequently, the JSON
serialization of every legacy schema report remains unchanged; no null or empty
assessment field is added.

For evidence leaves, the ordinary `CheckResult.verdict` is the cautious K3
projection:

| State | Projected verdict |
|---|---|
| `neither` | `Inconclusive` |
| `support_only` | `Pass` |
| `refute_only` | `Fail` |
| `both` | `Inconclusive` |

Refutations from an inconclusive `both` state remain in the evidence summary
and never enter the flattened failure-delta stream. This is an additive report
surface for opted-in schemas, not a reinterpretation of legacy `votes` or
`errors`.

Rust source compatibility is narrower than JSON compatibility. Adding optional
fields to the public `LeafSpec` and `CheckResult` structs requires downstream
code that constructs those structs exhaustively to add `evidence: None` or use
a constructor. Selta 0.2 MUST document this mechanical migration and release it
as a semver-visible interface change. It MUST NOT claim that Serde-defaulted
fields make Rust struct literals source-compatible.

Adding a default trait method does not require existing `ExtensionHost`
implementations to add a method body. No existing exhaustive match over
`PassFail` changes because that enum remains closed and unchanged.

## 7. Compatibility matrix

Revision-2 rows describe core/host behavior when the owning caller supplies an
explicitly revision-2-admitted schema. They do not imply that the current daemon catalog
can register or persist that schema; daemon promotion remains a separate gate.

| Runtime | Schema | Host | Required behavior |
|---|---|---|---|
| Selta 0.1 | revision 1 | legacy | Exact Selta 0.1 behavior |
| Selta 0.2 | revision 1 | legacy or native | Exact Selta 0.1 path; `assess` is never called |
| Selta 0.2 | revision 2, no `evidence` | legacy or native | Legacy path; `assess` is never called |
| Selta 0.2 | revision 2, `evidence: "cautious"` | in-process legacy | Default `assess` embeds `verify` |
| Selta 0.2 | revision 2, `evidence: "cautious"` | in-process native | Native `assess` result |
| Selta 0.2 | revision 2, `evidence: "cautious"` | RPC without assessor, correct `-32601` | One `assess` capability failure, then one legacy `verify` embedding |
| Selta 0.2 | revision 2, `evidence: "cautious"` | RPC native assessor | One native `assess`; no `verify` call |
| Selta 0.2 | revision 2, `evidence: "cautious"` | RPC timeout or any non-`-32601` failure | Operationally unavailable; no fallback |
| Selta 0.2 | revision 2, `evidence: "cautious"` | pre-upgrade RPC host that ignores unknown methods | Timeout and operationally unavailable; legacy schemas remain unaffected |
| Selta 0.1 | revision 2 | any | Revision or unknown-field admission rejection before execution |

The matrix applies equally to child-process and WebSocket transports because
both use the same RPC peer and method semantics.

## 8. Promotion tests

The feature cannot become a default dependency source until all of the following
pass:

1. The complete Selta 0.1 workspace suite passes unchanged.
2. Golden revision-1 schemas and reports remain byte-identical at every boundary
   that previously promised byte identity.
3. Revision-1 admission rejects `evidence`; revision-2 admission accepts only
   `"cautious"` and enforces every additional static rule.
4. Default in-process embedding maps valid pass and fail envelopes exactly and
   preserves usage and operational errors.
5. Native in-process assessment covers `neither`, `support_only`, `refute_only`,
   and `both`.
6. RPC tests prove fallback on structured `-32601` and no fallback on every
   other error code, timeout, malformed response, invalid usage, invalid
   refutation, cancellation, or disconnect.
7. SDK tests prove that a server using only initialize and verify observes the
   exact old extension declarations and verify results, with no assessor
   capability field, whether or not an assessor is registered. Host version is
   allowed to reflect the SDK release.
8. A real child-process SDK host passes both native assessment and legacy
   fallback cases.
9. A real WebSocket host passes the same cases and disconnect remains an
   operational error rather than semantic `neither`.
10. Evidence-summary serialization distinguishes semantic `neither`, `both`,
    and total unavailability while legacy reports omit the field entirely.
11. A call-count test proves that non-`-32601` errors never trigger a second host
    execution.
12. The rollback rehearsal in section 9 restores the revision-1 behavior and
    dependency pins.

## 9. Rollback

Rollback is designed around immutable schema versions and the unchanged legacy
path.

Before promotion:

- retain the accepted Selta 0.1 source and conformance identities;
- back up catalog data before adding revision metadata;
- record the latest revision-1 version of every schema that will receive a
  revision-2 successor; and
- keep host and SDK releases independently reversible.

To disable the feature while retaining the Selta 0.2 binary:

1. Reject new revision-2 registration at the owning boundary.
2. Pin every consumer to its recorded revision-1 schema version.
3. Continue executing revision-1 schemas through the untouched legacy path.
4. Leave revision-2 versions immutable and unavailable rather than rewriting or
   reclassifying them.

To roll the binary back to Selta 0.1:

1. Pin requests explicitly to the recorded revision-1 schema versions; an
   unversioned latest reference MUST NOT be used where a later revision-2
   version exists.
2. Restore the prior code and dependency source identities atomically.
3. Restore the pre-migration catalog backup if the old daemon cannot ignore the
   additive revision metadata.
4. Run the frozen revision-1 admission, protocol, report, RPC, server, and CLI
   suites before reopening execution.

No protocol rollback is required: Protocol-1 initialize, manifests, `verify`,
`Envelope`, and `PassFail` never changed. A newer SDK with registered assessors
may remain deployed because an old server never calls `assess`; rolling the SDK
back is optional unless its own release fails the legacy golden tests.

Assessment results are not stored in the legacy envelope cache, so rollback
requires no cache migration or invalidation. Revision-2 schemas and reports may
be archived for analysis, but they MUST NOT be relabelled as revision-1 results.

## 10. Non-claims

This compatibility strategy does not claim that a legacy pass is objective
truth or that a legacy failure is objective refutation. The default embedding
preserves exactly what the configured extension returned and exposes the more
cautious four-state composition selected by the schema author.

It also does not introduce protocol negotiation, assessor capabilities in
manifests, assessment caching, confidence scores, independence assumptions, a
second executor, or automatic schema promotion. Each would widen a frozen
boundary and requires a separately demonstrated need.
