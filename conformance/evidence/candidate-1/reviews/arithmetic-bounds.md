# Candidate-1 arithmetic bounds review

> **Status: reviewed for the frozen S2 profile set.** This is a durable
> conformance review, not normative evidence semantics. The language, phase
> schedule, counters, and errors remain those in
> [document 15](../../../../docs/15-evidence-language.md),
> [document 16](../../../../docs/16-evidence-semantics.md), and
> [document 20](../../../../docs/20-evidence-error-catalog.md).

## Claim and domain

For an admitted package in the candidate outer domain, and conforming
implementations of the exact descriptors in the
[presence](../../../../docs/19-presence-reference-profile.md),
[mechanics](../../../../docs/22-mechanics-conformance-profile.md),
[boundary](../../../../profiles/evidence/boundary-candidate-1/README.md), and
[positive-controls](../../../../docs/23-positive-controls-profile.md)
profiles, every actual resource quantity and every actual semantic-call fuel
sum is strictly below `SafeInt::MAX`.

This claim is deliberately conditional on conforming profile relations. It is
not an unconditional claim about an arbitrary host result or a conformance
measurement override. Candidate 1 charges an actual semantic output before
validating its contract, and it intentionally retains
`evaluation.arithmetic_overflow` as a defensive result at that boundary.

## Fixed constants

Let:

| Symbol | Value | Meaning |
|---|---:|---|
| `M` | `9,007,199,254,740,991` | `SafeInt::MAX` |
| `B` | `67,108,864` | outer raw package-byte ceiling |
| `V` | `1,000,000` | outer parsed JSON-value ceiling |
| `L` | `16,777,216` | outer bytes in one decoded string |
| `J` | `99,108,864` | conservative JCS package-byte bound |

The definition of `J` is:

```text
J = B + 32V = 99,108,864
```

Canonical string encoding cannot be longer than the already valid raw JSON
escape needed for that decoded string. Whitespace and noncanonical escapes
shrink. Reordering object members does not change their total length. Candidate
integer fields are nonnegative `SafeInt` values, but an open application value
may also carry a finite non-integral or negative I-JSON number. RFC 8785
serializes every such binary64 value with the finite ECMAScript shortest-number
form; including sign, decimal point, and exponent, that form is shorter than 32
bytes. Allowing 32 additional bytes for every parsed value is therefore
conservative even if every value were a number whose canonical spelling
expanded. Consequently the canonical package is at most `J` bytes, every input
subtree is at most `J` bytes, and the sum of the JCS byte lengths of all input
`TypedDocument` entries is at most `J`.

Every digest string and contract ID has fixed length. Every `DocumentRef`,
`FactRef`, and `SemanticBinding` consequently has a fixed representation
bound independent of application content.

## Input and graph counters

Each counted document, attempt, graph node, premise edge, target, grant,
assumption, decision, and scoped accounting record occupies at least one of
the `V` parsed values. Canonical sets prevent repetition from increasing a
logical count. Acyclic evidence depth cannot exceed the number of graph nodes.

The resulting counter bounds are:

| Counter or quantity | Upper bound |
|---|---:|
| input `documents` | `V = 1,000,000` |
| input `total_document_bytes` | `J = 99,108,864` |
| `attempts_total` | `V` |
| `graph_nodes` | `V` |
| `graph_edges` | `V` |
| `max_fan_in_observed` | `V` |
| `graph_depth` | `V` |
| relation-phase semantic calls | `V` |
| all semantic calls | `V + 2 = 1,000,002` |

The final two calls are the one interpreter and one projection call. This
bound follows the actual dispatch schedule, rather than the caller-selected
`max_semantic_calls` value.

The boundary profile's open application record does not weaken these bounds.
Its names and values remain inside `B`, `V`, `L`, and the outer depth ceiling,
and no boundary descriptor expands them into generated semantic output.

## Exact profile-output bounds

For the frozen descriptor set:

- exact claim theory, explicit assumption, extraction, operational,
  forwarding, checked-witness, and sequential-recovery outputs contain only a
  fixed number of digests and document references;
- disposition, stop, and attestation outputs are smaller fixed records;
- a declared attempt proposal is an echoed input typed value and is therefore
  no larger than `J`;
- the presence interpreter emits one target entry per target and, for each
  target, every matching node ID at most once in exactly one of the support or
  refute bases; and
- cautious projection emits one conclusion per target and at most two labels
  per conclusion.

Let `T` be the target count and `N` the graph-node count. The target-reference
objects in the basis and node objects in the graph are disjoint parsed values,
so `T + N <= V`. A presence-state target entry needs fewer than 512 fixed and
per-ID bytes under this accounting. Therefore:

```text
state_bytes <= 512(V + 1) = 512,000,512 < 8B
```

A projected conclusion with two labels needs fewer than 768 bytes, including
its separators and wrapper share. Therefore:

```text
projection_output_bytes <= 768(V + 1)
                        = 768,000,768 < 12B

conclusion_labels_total <= 2V = 2,000,000
```

Wrapping the state as one `TypedDocument` adds less than 512 bytes. Hence:

```text
documents_after_state <= V + 1 = 1,000,001

total_document_bytes_after_state
  <= J + 512(V + 2)
   = 611,109,888
```

All of these quantities are far below `M` before their basis limits are
compared.

### Document-23 scope correction

The positive-controls sequential basis now carries one independently
constructed Text `scope` reference. The seven assumptions use that reference,
and the recoverability claim refers to the completed sequential basis. This
removes the former identity back-edge without changing arithmetic growth.
It adds one fixed-size `DocumentRef` to `SequentialBasis` and its resolved rule
input. The sequential profile still has exactly seven assumptions, one
premise, fixed-shape attestation and recovery results, and no generated
collection proportional to application text. The bounds above are unchanged.

## Semantic-call fuel

Every candidate call costs:

```text
1 + UTF8_bytes(JCS(invocation)) + UTF8_bytes(JCS(output))
```

For the named relation-phase descriptors, a complete invocation is bounded by
`4J`. This covers the largest copied input combinations: a complete shallow
trace, a resolved response or witness, parameters, the document-23 expectation
and its repeated exception set, and fixed envelope fields. Every conforming
relation-phase output is at most `J`. Using the looser `8J + 1` bound per call
gives:

```text
all R2 fuel <= V(8J + 1)
            = 792,870,913,000,000
```

The interpreter invocation is also below `4J`, and its output is below `8B`:

```text
interpreter_call_fuel <= 4J + 8B + 1
                      = 933,306,369
```

The projection invocation is below `4J`, and its output is below `12B`:

```text
projection_call_fuel <= 4J + 12B + 1
                     = 1,201,741,825
```

Thus a complete conforming assessment satisfies:

```text
semantic_fuel
  <= 792,870,913,000,000
   +    933,306,369
   +  1,201,741,825
   = 792,873,048,048,194
   < M
```

No relation-, interpret-, or project-phase checked addition can overflow for
an actual conforming output in the frozen profile set.

## Outcome and error measurement

There is one state and one conclusion closure per target, so at most `2T`
closures. In one complete closure:

- evidence-node facts contribute at most `N` entries;
- atom qualification contributes at most another `N` attempt facts;
- derivation premises contribute at most `E <= V` entries;
- assumptions and grants contribute at most `V` digest strings each;
- semantic bindings contribute at most `V + 2` fixed records; and
- direct inputs and the target record have fixed size.

A `FactRef` or `SemanticBinding` is safely below 320 bytes including its array
separator, and a digest-list member is below 74 bytes. The resulting
conservative closure bound is `1536(V + 2)` bytes. Adding conclusions,
executions, resources, and wrapper overhead gives this complete concluded
outcome bound:

```text
2V * 1536(V + 2)
  + 768(V + 1)
  + 256(V + 2)
  + 4096
= 3,072,007,168,005,376
< M
```

Document 20 returns at most 256 error records. A normalized pointer is at most
2,048 UTF-8 bytes; even allowing sixfold JSON escaping plus all fixed fields,
one error record is below 12,800 bytes. At most `V + 2` execution records are
possible, each below 256 bytes. Therefore an ordinary error outcome is below:

```text
256(V + 2) + 256 * 12,800 + 4096
= 259,281,408
< 4B
< M
```

The fixed oversized-outcome replacement is below 4 KiB. O0 can therefore
measure every ordinary concluded or error outcome in a `SafeInt` before
applying the 1,048,576-byte replacement rule. O1's retained-map comparison and
insertion add no resource quantity.

## Why unconditional unreachability is false

The outer package ceilings constrain submitted input, not a value returned by
a violated host-conformance precondition or an injected conformance control.
Document 16 dispatches first and charges the actual output before checking its
sole output contract. Several output schemas also contain arrays without a
schema maximum, because their exact semantic relation supplies the real bound.

Consequently an arbitrary host result can make
`UTF8_bytes(JCS(output))` large enough that the step-3 fuel addition overflows
before `evaluation.invalid_output` or semantic comparison. The same defensive
branch exists in all three semantic phases. This does not contradict the proof
above: such a result is not the output of the frozen conforming relation.

Candidate 1 must therefore not classify `evaluation.arithmetic_overflow` as
unconditionally unreachable. A core semantic-output ceiling would change
dispatch precedence, charged-work commits, the core source identities, and
every dependent environment identity. It is unnecessary for S2 evidence.

## Conformance-control classification

The conformance-only control specified in
[document 21](../../../../docs/21-s2-conformance-plan.md) and the
[candidate conformance contract](../README.md) is:

```text
semantic_output_bytes: [{
  call_ordinal,
  semantics,
  utf8_bytes: SafeInt
}]
```

An entry exact-matches one scheduled call and replaces only the
`UTF8_bytes(JCS(output))` quantity used in dispatch step 3. It does not replace
or modify the actual output. The actual value remains available for contract
verification and comparison if step 3 succeeds. A call cannot also have a
`semantic_outputs` override, and every entry must be used exactly once.

Set `utf8_bytes = M`. Since every invocation has positive encoded length,
computing `1 + invocation_bytes + M` overflows `SafeInt`. This exercises the
real checked-add branch without a petabyte artifact. The required cases are:

| Required case | Target call | Error phase |
|---|---|---|
| `mechanics-arithmetic-overflow-relation` | one R2 call | `relation` |
| `mechanics-arithmetic-overflow-interpret` | interpreter | `interpret` |
| `mechanics-arithmetic-overflow-project` | projection | `project` |

Each oracle requires:

- code `evaluation.arithmetic_overflow`;
- path `/generated/resources/semantic_fuel`;
- fixed message `candidate resource arithmetic overflowed SafeInt`;
- the target dispatch counted in `semantic_calls`;
- the target implementation recorded in `executions`;
- `semantic_fuel` left at its prior representable total; and
- a basis fuel limit at least the prior representable subtotal but below the
  injected hypothetical mathematical total, so the earlier calls remain
  admissible while checked overflow wins before the fuel-limit comparison; and
- no later generated resource or phase commit after the failed step 3.

The package oracle cannot observe whether an implementation improperly visited
output-contract validation or semantic comparison after overflow: the control
preserves a valid actual output and cannot overlap `semantic_outputs`. Skipping
those internal visits is a normative dispatch rule to inspect in the two model
implementations; the fixtures claim only the observable error, counters,
executions, and absence of later commits.

The phase-specific generated commits are also exact:

- relation overflow occurs before any state generation;
- interpreter overflow occurs before T1, so state document, state bytes, and
  generated document bytes do not commit; and
- projection overflow occurs after the state and interpreter resources have
  committed, but before `conclusion_labels_total` commits.

The error classification is therefore:

```text
evaluation.arithmetic_overflow -> control-only via semantic_output_bytes
```

Within the admitted/conforming implementation domain it is not ordinary-wire
reachable. Outside that host precondition it remains the specified defensive
result, which is precisely the boundary the conformance control reproduces.
