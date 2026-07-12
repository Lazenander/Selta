# Candidate-1 evidence conformance records

> **Status: S2 conformance support, not candidate evidence semantics.** The
> normative language, assessment judgment, error behavior, and conformance
> protocol remain in documents 15, 16, 20, and 21. Nothing in this directory
> is a product API or a second definition of those rules.

This directory defines the Selta shapes needed to inventory atomic S2
cases, keep their withheld oracles separate, state law and countermodel
coverage, and encode the finite exact countermodels from document 14:

- `schemas/manifest-input.schema.json` — the small authored choices from which
  the final manifest is derived;
- `schemas/manifest.schema.json` — the arbiter-side artifact, context, case,
  law, named-case, and error-coverage index;
- `schemas/case.schema.json` — one environment-admission, assessment,
  finite-world, or comparison request;
- `schemas/oracle.schema.json` — the separately held expected response;
- `schemas/request.schema.json` — the private model's exact stdin request;
- `schemas/response.schema.json` — the private model's exact stdout response;
- `schemas/finite-world.schema.json` — one small exact finite calculation;
- `schemas/finite-result.schema.json` — its canonical result ledger;
- `schemas/artifact-set.schema.json` — an exact path-and-byte set;
- `schemas/kit-index.schema.json` — the producer-neutral model input index;
- `schemas/prediction-ledger.schema.json` — one model's complete executions;
- `schemas/completion-ledger.schema.json` — the arbiter's minimal all-pass
  claim;
- `schemas/runtime-evidence.schema.json` — the actual sealed process launcher
  and probe record; and
- `schemas/seal.schema.json` — the freeze, prediction, and completion records.

[SEALING.md](SEALING.md) fixes their identities, acyclic lifecycle, withheld
split, and invalidation rules.

The schemas describe closed wire shapes. The relational checks below are part
of conformance-manifest admission by the S2 harness. They are not evidence DSL
features and MUST NOT be implemented in the candidate assessment core.

## Artifact and context rules

`bytes_sha256` is the lowercase display of ordinary SHA-256 over the exact file
bytes:

```text
sha256:<64 lowercase hexadecimal digits>
```

It is repository-integrity metadata, not any domain-separated identity from
document 16. An artifact path is relative to the repository root, uses `/`,
and contains neither `.` nor `..` segments. The harness checks the bytes before
parsing them. Each artifact has a canonical `admissions` set. An empty set
means exact raw bytes only. A `selta_schema` admission must pass the candidate's
stable pure-builtin profile. Every `selta_value` admission must verify in
strict `Input::Value` mode under the artifact named by its `schema_source`.
The same bytes may therefore be admitted as a schema, under several value
schemas, or both without duplicating the artifact.

Artifact entries are unique by raw digest, ordered by `bytes_sha256`, and use
the lexically first repository path among equal-byte aliases. Their admission
rows are unique and ordered by `(kind, schema_source)`, treating the absent
`selta_schema` `schema_source` as the empty string. Resolver rows are ordered
by `(digest, domain, source)`, contexts and cases by `id`, laws by `number`, and
named rows in the order fixed by document 17. Error rows use the phase rank
`parse < schema < identity < relation < interpret < project < output`, then
`code`; their case lists and every other set-like case-ID array use lexical
order. Equal byte digests reuse one artifact entry; unequal available bytes
under one byte digest fail manifest admission.

Each context supplies exact environment source bytes and a digest/domain/source
resolver. Every resolver source and schema source resolves through the artifact
inventory. Context IDs and case IDs are lowercase kebab names. Every case has
exactly one record and one oracle, their embedded case IDs agree with the
inventory, and every reference closes. The fifteen law rows occur once in
numeric order and have non-empty positive and falsifier sets. The sixteen named
rows occur once in document-17 order and have non-empty witness sets. Error
coverage is complete for document 20 when keyed by `(phase, code)`; messages,
pointers, precedence, counters, and expected execution sets are read from the
referenced oracle packages rather than copied into the manifest.

The repository manifest is arbiter-side. An implementer kit is generated from
the transitive context and case-input closure and omits the manifest's oracle
references, oracle records, expected artifacts, and their byte digests.

## Case execution

Case operations have only the meanings fixed by document 21:

- `admit_environment` invokes the private conformance admission request for
  the named context;
- `assess` invokes the assessment request with the sealed executable's own
  implementation digest;
- `finite_world` invokes the generic exact evaluator below; and
- `compare` observes already completed cases without executing assessment
  semantics.

The request and response schemas are only the closed private protocol boundary
from document 21. Encoded source fields use non-empty canonical Base64. A
`returned` response admits an I-JSON `package` value at this outer boundary;
the arbiter independently verifies it under the candidate package schema and
resolved document contracts before using it.

If an assessment `controls` object is present, all six arrays are present,
even when empty; the separate `forbidden_boundary` member remains optional.
`call_ordinal` is zero-based in document 16's dispatch order.
An input/state `identity_results` entry carries a non-empty canonical-Base64
literal preimage and replaces the digest for exactly one match at its named
insertion boundary. The encoded bytes include the complete document-16 domain
tag, `0x00` separator, and JCS or raw suffix actually passed to SHA-256.
Multiple entries may map different preimages to one digest, so both collision
and fixed-point/cycle identity cases are expressible.

An outcome entry instead carries `preimage_template_base64` and uses only the
fixed `__SELTA_S2_REQUEST_IMPLEMENTATION__` sentinel under the structural
substitution and exact-use rules in document 21. This removes the executable-
identity cycle without modifying any actual candidate outcome.

Semantic outputs, semantic-output-byte measurements, semantic-read attempts,
and verification faults match exactly one scheduled call or defensive
boundary. A `semantic_output_bytes` entry
replaces only the `UTF8_bytes(JCS(output))` quantity used in dispatch step 3;
the actual output remains unchanged for contract verification, comparison, and
every other purpose. A call cannot occur in both `semantic_outputs` and
`semantic_output_bytes`. A
`semantic_reads` entry injects exactly one attempted `DocumentRef` dereference
through the dispatch-time resolver at its named call and does not itself expose
the value. A matching `forbidden_reads` entry returns
`{ kind: "forbidden_read", reference: DocumentRef }` before the value is
exposed. This is the protocol's only trap variant. Every entry in the six
arrays must match exactly once; unused, multiply matched, overlapping, or
out-of-scope entries fail the conformance case.

`forbidden_boundary: "concluded_outcome_insertion"` is an optional negative
assertion rather than an ordinary control entry. Zero visits satisfies it at
assessment completion. Its event is the start of O1 after O0 has accepted the
ordinary concluded outcome's measured size and before retained-map comparison
or insertion. A visit fails the private execution before O1 effects. Digest
work performed while O0 measures or replaces an oversized outcome is not that
event. The assertion never enters candidate values, semantics, identities,
resources, or the product interface.

A comparison observation uses an empty pointer for the whole selected view.
`package_jcs` always uses the empty pointer and denotes JCS bytes of the returned
package, not raw response formatting. Other pointers address the exact
`ConformanceProjection`, finite result, or trap value. In `per_model` scope,
`current` selects the model being checked and `arbiter` selects a model-neutral
finite result. In `cross_model` scope, assessment observations select
`model_a` or `model_b`; `arbiter` remains model-neutral. Comparison dependencies
are acyclic.

An oracle's returned package is the sole source of expected candidate state,
closures, conclusions, resources, errors, and semantic executions. The harness
either compares its complete JCS package bytes or derives the exact non-wire
`ConformanceProjection` in document 16. No second serialized projection or
convenience copy of an expected error is permitted.

## Finite evaluator

The finite evaluator is deliberately weaker than a programming language.
Values are booleans, safe integers, or bounded tokens. Nonnegative arbitrary
precision integers use canonical decimal strings; weights and denominators are
strictly positive, and every such integer has at most 4096 decimal digits.
Every probability numerator is at most its denominator. Input and result
fractions are reduced, including the base fraction of a factored result.

For a `world_table`:

- row IDs and fact names are unique and canonical;
- each DNF selector is `{ any_of: [{ all_of: [{ fact, equals }] }] }`;
  `all_of` clauses are unique and ordered by `(fact, JCS(equals))`, and
  `any_of` conjunctions are unique and ordered by their JCS bytes;
- a clause resolves its fact exactly once in each evaluated row and compares
  the scalar by exact JSON type and value; a missing or duplicate fact makes
  the finite case invalid;
- an empty `all_of` is true and an empty `any_of` is false; a selector matches
  a row when any of its conjunctions is true;
- `weighted_event` forms event and `given` row sets from those selectors,
  using all rows when `given` is absent, requires the event set to be a subset
  of the non-zero-weight `given` set, and returns their reduced exact weight
  ratio;
- a world-table case is relationally invalid if any exact weight aggregate
  required to answer it exceeds 4096 decimal digits;
- explicit row subsets occur only on `distinct_values` and `argmax`; every row
  ID there resolves and each list is a canonical set;
- every row selected by `distinct_values` or `argmax` contains exactly one
  value for every field read by that query;
- `distinct_values` returns the number of distinct canonical tuples over its
  named fields, using all rows when `rows` is absent; and
- `argmax` requires a safe-integer score field, chooses the largest score, and
  breaks a tie by lexical row ID before returning the named projected fact.

For `repeat_event`, every trial count is at least one and each trial computes
`1 - (1 - p)^n`. A `reduced` result is admissible only when `count *
max(decimal-digit lengths of denominator and denominator-numerator) <= 4096`;
the evaluator checks that bound from lengths and arithmetic without expanding
either power. Otherwise the requested representation must be `factored`.
`factored` produces the exact `one_minus_power` form with the reduced failure
probability as its base and `n` as its exponent, and never expands the power.

For `transition`, listed states are unique, the initial state is listed, and
the transition table is a total deterministic function on the listed states.
An observation returns the state after exactly its stated number of
applications. Queries and results are unique and ordered by name.

Manifest admission relates each finite result one-to-one by name with the
program's queries, trials, or observations: no name may be omitted or added,
and result entries are in canonical name order. A `weighted_event` produces a
`rational`, `distinct_values` a `safe_int`, and `argmax` or `transition` the
same scalar kind as the projected or observed value. A `repeat_event` trial
produces a `rational` when its requested representation is `reduced` and a
`one_minus_power` when it is `factored`.

These operations are sufficient for the finite probability tables, recursive
fixed point, proxy selection, observational indistinguishability, optional
stopping, and adaptive-holdout calculations in document 14. Acquisition,
checked-witness, and assumption-recoverability witnesses remain ordinary
assessment cases under their own conformance environments.

## What remains prose

Law meaning, countermodel interpretation, proof sketches, empirical transfer
limits, privacy and platform analysis, independence-review reasoning, and the
S4/L5 boundary remain in the numbered documents and review records. These
schemas do not restate candidate call schedules, identity formulas, trust
semantics, error text, pointer rules, or projection semantics.
