# Presence assessment

Status: implemented Selta 0.2 core candidate. Promotion gates remain pending,
and daemon/catalog promotion is outside this slice.

This document specifies the minimum complete addition that lets one verifier
preserve support, refutation, conflict, and semantic abstention. It extends the
existing engine; it does not introduce another executor, evidence graph, or
protocol-wide result model.

Normative terms **MUST**, **MUST NOT**, **SHOULD**, and **MAY** have their usual
RFC 2119 meanings.

## 1. Schema surface

Evidence-preserving execution is opt-in on a verifier leaf:

```json
{
  "ext": "semantic_judge",
  "config": { "criterion": "the claim is supported" },
  "evidence": "cautious",
  "sampling": {
    "samples": 5,
    "depth": 1,
    "min_valid": 3
  }
}
```

The corresponding additive schema field is:

```rust
pub enum EvidenceProjection {
    Cautious,
}

pub struct LeafSpec {
    pub ext: String,
    pub config: Value,
    pub sampling: Option<Sampling>,
    pub evidence: Option<EvidenceProjection>,
}
```

`evidence` defaults to `None` and is omitted when serialized. Selta 0.2 admits
exactly the string `"cautious"`.

When `evidence` is absent, the leaf MUST use the Selta 0.1 verification path
unchanged. When it is present, the leaf MUST use the assessment path specified
below. An authored `sampling.vote` MUST NOT occur on an evidence leaf: count
voting and cautious evidence projection are alternative semantics. The other
sampling fields retain their existing acquisition and budget meanings.

Within one `all_of`, `any_of`, or `not` verifier subtree, either every leaf MUST
omit `evidence` or every leaf MUST select `"cautious"`. Separate entries in a
node's `verify` array MAY use different modes because they produce separate
checks. Strict admission MUST reject a mixed subtree.

## 2. Host boundary

The protocol-1 verification envelope remains unchanged. A separate result
type carries a semantic assessment:

```rust
pub struct AssessmentEnvelope {
    pub support: bool,
    pub refute: Option<WireDelta>,
    pub usage: Option<WireUsage>,
}
```

The representation has exactly four meanings:

| `support` | `refute` | state |
|---|---|---|
| `false` | absent | `neither` |
| `true` | absent | `support_only` |
| `false` | present | `refute_only` |
| `true` | present | `both` |

A present refutation MUST contain the normal non-empty `WireDelta.message`.
Thus a refutation always remains inspectable, while support requires no new
payload convention.

`ExtensionHost` gains one default method:

```rust
async fn assess(
    &self,
    call: HostCall<'_>,
) -> Result<AssessmentEnvelope, String>;
```

Its default implementation calls `verify` and embeds a valid legacy result:

- `Pass` without a delta becomes `support_only`;
- `Fail` with a delta becomes `refute_only`;
- a malformed legacy envelope remains an operational error; and
- a host error remains an operational error.

A native assessor overrides `assess` and may return all four states in one
call. `Ok(AssessmentEnvelope { support: false, refute: None, ... })` is a
successful semantic abstention. `Err(...)` means that no semantic assessment
was obtained. These cases MUST NOT be identified with one another.

For JSON-RPC hosts, `assess` is an optional method using the existing
`VerifyParams` request shape and returning `AssessmentEnvelope`. On JSON-RPC
method-not-found only, `RpcHost` MUST fall back to `verify` and apply the legacy
embedding. Other RPC failures MUST remain operational failures. This leaves the
protocol-1 `verify` request and `Envelope` wire contract intact.

## 3. Semantic state and algebra

```rust
pub enum EvidenceState {
    Neither,
    SupportOnly,
    RefuteOnly,
    Both,
}
```

Write a state as a pair `(S, R)` of support and refutation presence:

```text
neither       = (0, 0)
support_only  = (1, 0)
refute_only   = (0, 1)
both          = (1, 1)
```

Repeated assessments of the same leaf combine by knowledge join:

```text
(S1, R1) ⊔k (S2, R2) = (S1 or S2, R1 or R2)
```

Knowledge join is associative, commutative, and idempotent, with `neither` as
its identity. Repetition may change observation counts, but it MUST NOT by
itself strengthen the state or its projection.

Evidence-mode combinators use the four-valued truth operations:

```text
not(S, R)      = (R, S)
all_of(states) = (and S_i, or R_i)
any_of(states) = (or S_i, and R_i)
```

Composition distinguishes raw observations from a usable child conclusion. A
child that is skipped or ends with an operational error, including a failed
`min_valid` completion gate, has no usable conclusion. Its successful envelopes
still contribute their raw category counts, and failed attempts contribute
unavailable counts, to aggregate evidence reporting; unscheduled budget-short
slots contribute unavailable counts as specified below. `all_of` and `any_of`
substitute `neither` for that child during state composition, while `not` has no
completed inner state. No parent promotes the incomplete child's refutations.
The operational failure remains in the report-wide error stream.

Missing polarity does not impose a separate K3 veto after the FOUR operation.
A parent may therefore still be decisive when the result is insensitive to the
missing conclusion: a completed refuting child can refute an `all_of`, and a
completed supporting child can support an `any_of`. Conversely, an incomplete
child prevents support for that `all_of` and refutation of that `any_of`.
`not.message` supplies the refuting detail only when completed support for the
inner check becomes refutation of the negated check.

## 4. Sampling and operational completion

Every successful `AssessmentEnvelope`, including `neither` and `both`, is one
valid assessment. A host error, timeout, malformed envelope, exhausted budget,
or invalid returned refutation is unavailable and is not a semantic state.

Evidence sampling retains the existing bounded acquisition discipline:

- `samples` is the target number of valid assessments;
- while that target is unmet, an operational failure MUST be retried when the
  request budget and the existing `2 * samples` attempt ceiling permit;
- every failed attempt is counted as unavailable even if a retry succeeds;
- a requested assessment slot that cannot be scheduled because the shared
  sample budget is already short is also counted as unavailable, but it is not
  an attempted host call and contributes no host usage;
- the operational completion gate is explicit `min_valid`, or otherwise
  `ceil(samples / 2)`, with a minimum of one; and
- request depth, sample, and deadline bounds remain authoritative.

The completion gate counts successful envelopes, not support votes. Failing
the gate can only make that check's projected result `Inconclusive`; it MUST NOT
alter its recorded local semantic state or create refutation. At the next
combinator, however, the check has no usable completed conclusion as specified
in section 3. A quorum error SHOULD distinguish budget exhaustion from host or
envelope failure. Counts are observations, not claims of independence,
confidence, or statistical amplification.

Evidence-mode calls bypass the legacy `Envelope` cache in Selta 0.2. This is a
safe performance limitation, not a semantic limitation. Usage accounting,
deadlines, cancellation, and monitoring apply exactly as at the existing host
boundary.

## 5. Report contract

An evidence-mode check carries one optional summary:

```rust
pub struct EvidenceSummary {
    /// Absent when no semantic assessment completed.
    pub state: Option<EvidenceState>,
    pub neither: u32,
    pub support_only: u32,
    pub refute_only: u32,
    pub both: u32,
    pub unavailable: u32,
    pub refutations: Vec<Delta>,
}

pub struct CheckResult {
    // existing fields remain unchanged
    pub evidence: Option<EvidenceSummary>,
}
```

`CheckResult.evidence` is absent on legacy checks. It is present on an
evidence-mode check even when every attempt was unavailable. Within the
summary, `state` is absent until at least one semantic assessment completes.
Consequently the following records are distinct:

```json
{
  "verdict": "inconclusive",
  "evidence": {
    "state": "neither",
    "neither": 1,
    "support_only": 0,
    "refute_only": 0,
    "both": 0,
    "unavailable": 0
  }
}
```

```json
{
  "verdict": "inconclusive",
  "error": "assessor timed out",
  "evidence": {
    "neither": 0,
    "support_only": 0,
    "refute_only": 0,
    "both": 0,
    "unavailable": 1
  }
}
```

Category counts preserve whether one assessor returned `both` or separate
assessors returned opposing single-polarity states. Counter addition MUST be
checked for overflow. A leaf's `state` is the knowledge join of its successful
assessments. A combinator's state uses the truth operations above over usable
completed child conclusions; its counters are the checked sums of all native
descendant assessment outcomes, including observations retained by an
incomplete child, and are not relabelled by the combinator. If no usable
descendant conclusion exists, its state is absent. Otherwise a child without a
usable conclusion contributes no polarity to the truth operation.

Refuting details are normalized to semantic `Delta` values, kept in stable
observation order, and deduplicated only when the normalized values are equal.
Only details from usable completed children may be promoted into a parent
summary. They live in `EvidenceSummary.refutations` whenever the resulting
state has a refuting component. `all_of` retains details from completed
refuting children. `any_of` retains child refutations only when every child has
a completed refuting component. `not` uses its declared `message` as the
outward refuting detail when the completed inner state has support; inner
refutations support the negation and are not outward refutations.

The cautious projection is:

| semantic state | `CheckResult.verdict` |
|---|---|
| `neither` | `Inconclusive` |
| `support_only` | `Pass` |
| `refute_only` | `Fail` |
| `both` | `Inconclusive` |
| no completed assessment | `Inconclusive` |

If the operational completion gate fails, it overrides `Pass` or `Fail` to
`Inconclusive` without changing the summary.

`CheckResult.error` explains only an `Inconclusive` check. If FOUR composition
is decisive despite an incomplete child, the parent check has no local error;
the child's operational failure remains observable through aggregated
`EvidenceSummary.unavailable` and the report-wide `errors` collection.

Only a final evidence check whose verdict is `Fail` promotes its refuting
details into `CheckResult.deltas`, from which the report's top-level flattened
deltas are collected. For `both`, `neither`, an unavailable result, or any
other inconclusive evidence check, refuting details MUST remain solely inside
`EvidenceSummary.refutations`; they MUST NOT appear as top-level failure
deltas. Unrelated structural or legacy failures retain their normal deltas.

`fail_fast` responds only to the projected `Fail` verdict. It MUST NOT stop on
`both`, `neither`, or operational unavailability.

## 6. Compatibility and invariants

An implementation conforms only if all of the following hold:

1. Omitting `evidence` preserves Selta 0.1 execution and serialized reports.
2. `verify`, `Envelope`, `PassFail`, and their RPC method remain unchanged.
3. Operational failure never creates support, refutation, or semantic
   `neither`.
4. One native assessment can express each of the four states.
5. Conflict and abstention remain distinct even though both project to
   `Inconclusive`.
6. Knowledge join obeys associativity, commutativity, and idempotence.
7. `not` swaps polarity, and `all_of`/`any_of` obey the equations in section 3.
8. Multiplicity affects counters only; no independence is inferred.
9. Every refuting component has an inspectable refuting detail.
10. Operational completion may withhold a conclusion but may not rewrite
    evidence.
11. Inconclusive refutations remain in the evidence summary rather than the
    flattened failure-delta stream.
12. All counters, budgets, and usage totals use checked arithmetic.
13. Incomplete-child observations remain reportable but cannot act as a child
    conclusion or supply outward refutations.
14. A decisive cautious parent has no `CheckResult.error`; child operational
    failures remain visible in aggregate evidence and report-wide errors.

## 7. Non-goals

Selta 0.2 presence assessment does not define:

- evidence graphs, derivation languages, trust systems, or provenance stores;
- confidence scores, learned calibration, independence assumptions, or
  statistical amplification;
- a second executor, reference model, manifest ceremony, or sealing workflow;
- new daemon routes, persistence, queues, or UI policy;
- caching of assessment envelopes; or
- automatic conversion among evidence modalities.

Those features require separate demonstrated needs. This contract adds only
the smallest complete semantic distinction that the legacy binary verifier
cannot express.
