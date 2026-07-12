# Real-Codex vague-verifier pilot protocol

Status: formal-pilot preregistration draft. Engineering-only fixtures and harness
work may coexist with this file, but they do not instantiate or validate the formal
pilot. Changing a frozen rule requires a new protocol revision and a new held-out set.

## 1. Question and claim boundary

The pilot evaluates whether a short, domain-neutral prompt can identify textual
evidence concerning a supplied claim while preserving four distinct outcomes:
support only, refutation only, both, and neither.

It does not test whether the underlying claim is true. It tests whether the
assessor identifies evidence present in the supplied text under the annotation
contract. Outside knowledge, source reputation, missing context, and mere
plausibility are not evidence for this experiment.

This pilot selects a prompt for further testing. With only 24 held-out cases it
cannot establish a publication-grade performance improvement, calibrated
confidence, source independence, or semantic soundness.

## 2. Response contract

The model returns one closed JSON object with two arrays:

```json
{"support": ["short exact quotation"], "refute": []}
```

`support` contains text spans presented as evidence for the claim. `refute`
contains text spans presented as evidence against it. The derived state is:

| Support array | Refute array | State |
|---|---|---|
| empty | empty | `neither` |
| non-empty | empty | `support_only` |
| empty | non-empty | `refute_only` |
| non-empty | non-empty | `both` |

There is no authored `state` or confidence field. This removes a redundant value
that could disagree with the evidence collections and avoids treating an
unexplained scalar as assurance.

A response is admissible only when:

1. it satisfies `schemas/response.schema.json`;
2. each array contains at most three strings;
3. every string is non-empty, contains at most 160 Unicode scalar values, and is
   unique within its array; and
4. after JSON decoding, every string occurs contiguously and byte-for-byte in
   the original UTF-8 text; and
5. no exact quotation occurs in both arrays. Distinct overlapping excerpts are
   allowed.

Schema failure, an invented or altered quotation, process failure, timeout, and
missing output are operational failures. They never become `neither`, support,
or refutation. Exact quotation validity is mechanical; whether a valid quotation
really bears the claimed polarity remains part of semantic evaluation.

The JSON Schema is only Codex's structured-output guard. Codex's accepted schema
subset does not permit `uniqueItems`, so uniqueness and cross-side disjointness
are deliberately absent from that transport schema. The raw response is first
admitted by the canonical Selta schema; deterministic host checks then enforce
within-array uniqueness, cross-side disjointness, and quotation membership on
the Selta-admitted value. That fully checked value is the scoring input.

## 3. Corpus and split

No **formal pilot corpus** has been created. The repository contains an
author-constructed development fixture and a commitment. Separately prepared,
unlabeled engineering holdout inputs remain withheld outside the checkout until
the prompt is frozen. These artifacts exist to develop and exercise plumbing. They do
not have the three blinded human annotations required below, must not be substituted for
the formal split, and cannot support a semantic-performance or prompt-quality claim.
`corpus/ENGINEERING-NOTICE.md` records the development fixture's boundary; the
unannotated holdout inputs are likewise engineering-only.

The first formal pilot corpus will contain exactly 48 cases:

- 24 development cases;
- 24 held-out cases;
- in each split: 6 `support_only`, 6 `refute_only`, 6 `both`, and 6 `neither`;
- in each split and state: 3 clear and 3 boundary cases; and
- in each split: two claim/domain families of 12 cases each.

Development and held-out claim/domain families must not overlap. A source,
thread, author, document, paraphrase family, or templated near-duplicate group
belongs wholly to one split. Splitting happens at the group level before prompt
development. Stable opaque case IDs are assigned only after deduplication.

Three people annotate every case independently and without model predictions.
Each annotation records support presence, refutation presence, selected text
spans, and a short reason. Each distinct evidence unit receives one minimal
sufficient canonical exact span; an exact span may not be assigned to both
polarities. Valid unmatched alternatives remain conservative
`semantic_unaligned` or manual-adjudication cases. Representing alternatives
would require explicit equivalence classes and is outside this slice. The raw
144 annotation records remain available.
An adjudicator resolves disagreement into the two-bit evaluation oracle while
retaining the annotation distribution and adjudication note.

Balancing the four states makes this a diagnostic corpus. Pooled scores must not
be presented as deployment-prevalence estimates. Results are also reported by
state, clarity, and domain family.

Corpus material must be lawfully usable and stripped of unnecessary personal
data. Redaction must occur before annotation and must not change the evidence
needed for the case.

## 4. Prompt development

`prompts/p0.txt` is the baseline. At most one later prompt, `p1`, may be
created after inspecting development results. Tuning is permitted only if `p0`
has at least four oracle-aligned semantic errors spanning both development
domains and both clarity strata, and those errors support one general mechanism
rather than mainly exposing oracle disagreement. Otherwise `p0` wins without a
new prompt.

Smoke 002 failed at transport-schema validation before any judgment. Because no
development or semantic result existed, the pre-development baseline was
narrowly clarified from `Do not repeat a quotation across sides.` to the more
general `Do not repeat quotations.` This reduced `p0` from 89 to 86 words and
aligns it with the unchanged deterministic uniqueness and disjointness rules; it
is not a development-tuned `p1` attempt.

Every prompt must:

- remain zero-shot and domain-neutral;
- contain at most 90 words, excluding the case payload;
- contain no development quotation, case ID, source name, domain-specific label,
  or held-out information; and
- differ for one recorded, falsifiable error hypothesis at a time.

All attempted prompts and their stated hypotheses remain in the research
history, including prompts that perform worse. Prompt selection uses development
data only. The prompt author may not inspect held-out wording, annotations, or
labels before the selected prompt and its digest are frozen.

## 5. Real-Codex execution

Before the first call, the run manifest freezes the Codex CLI version, exact
model identifier, configuration, prompt digest, input and schema digests, random
ordering seed, concurrency, and command-line flags. There is no model sweep in
this pilot. The current engineering runner is qualified against Codex CLI
`0.144.1`; changing that version requires a new smoke and manifest.

Each case is one fresh, ephemeral invocation in an otherwise empty temporary
working directory. The runner snapshots inputs, both schemas, and prompts before
calls. It gives every child a fresh `CODEX_HOME` that starts with only a private
copy of `auth.json`, removes inherited `CODEX_*`, `OPENAI_*`, and `CHATGPT_*`
variables, and retains raw event, stderr, and response artifacts. The intended
command surface is:

```text
codex exec --ephemeral --ignore-user-config --ignore-rules --strict-config \
  --skip-git-repo-check --sandbox read-only -C <empty-directory> \
  --output-schema <snapshotted-response-schema> --json -m <pinned-model> \
  --config model_reasoning_effort=\"<pinned-effort>\" \
  --config web_search=\"disabled\" --config approval_policy=\"never\" \
  --config skills.include_instructions=false \
  --config skills.bundled.enabled=false \
  --config orchestrator.skills.enabled=false \
  --config include_environment_context=false \
  --config include_permissions_instructions=false \
  --config include_collaboration_mode_instructions=false \
  --config tools.experimental_request_user_input.enabled=false \
  --config notify=[] \
  --config features.multi_agent_v2.root_agent_usage_hint_text=\"\" \
  --config features.multi_agent_v2.multi_agent_mode_hint_text=\"\" \
  --config features.multi_agent_v2.max_concurrent_threads_per_session=1 \
  --config instructions=\"Follow the user instruction exactly.\" \
  --disable plugins --disable apps --disable shell_tool \
  --disable image_generation --disable goals --disable hooks \
  --disable personality --disable multi_agent --disable shell_snapshot \
  --output-last-message <raw-response-path> -
```

This centralized list is the source-traced minimum of direct optional gates for
CLI `0.144.1`, with `multi_agent` retained as declared intent and
`shell_snapshot` as independent process isolation. It is not a claim that one
parent flag disables every descendant or that `codex exec` has no core or
model-selected tool schemas. Codex exposes skill, orchestrator, environment,
permission, collaboration, request-input, notification, and shell-snapshot
channels through separate configuration paths, so each is pinned explicitly
rather than hidden behind a larger prompt.

Terra/Sol model metadata still selects multi-agent v2 despite `--disable
multi_agent`. Its two model-visible hint strings are therefore frozen empty, and
its session capacity is pinned to one: the root consumes that slot, preventing
an accidental subagent from spending tokens before rejection. This containment
does not remove schemas. Strict event auditing remains the actual no-tool-use
invariant: any tool event invalidates the invocation.

The fixed neutral harness instruction is exactly `Follow the user instruction
exactly.` Its bytes and SHA-256 digest are recorded in the manifest. It is a
harness constant, is never tuned, and is not a candidate prompt; `p0` remains
the only task prompt optimized under section 4.

The claim and text are supplied directly in the case prompt after `p0`; corpus
paths and oracle paths are never exposed. Event JSONL is retained. Any tool call
invalidates that invocation because the assessor was granted only the supplied
text.

`runs/smoke-terra-low-001` is an immutable, non-evidentiary infrastructure
record. It failed operationally because Codex CLI `0.142.5` was too old for
`gpt-5.6-terra`, and it exposed remote plugin synchronization into a fresh
`CODEX_HOME`. It made no development-corpus calls and supplies no semantic or
prompt-quality evidence. The runner was revised and the CLI upgraded to
`0.144.1`; the failed bundle is retained unchanged rather than repaired in place.

`runs/smoke-terra-low-002` is also immutable and non-evidentiary. Codex CLI
`0.144.1` accepted the isolated command and configuration, but the API rejected
the transport output schema because `uniqueItems` is unsupported. The failure
occurred before model judgment, made no development-corpus calls, and supplies
no semantic or prompt-quality evidence. The bundle remains unchanged; the live
transport schema alone drops `uniqueItems`, while post-Selta deterministic
uniqueness and disjointness checks remain authoritative.

Calls are run in a precommitted randomized order with at most four concurrently.
There is exactly one scored attempt per prompt-case pair. Semantic failure,
malformed output, or an undesirable answer is never retried. A diagnostic retry
after a transport failure, if performed, is recorded under a new attempt ID and
cannot replace or alter the scored first attempt.

### Call budget

- Development: at most 2 prompts x 24 cases = 48 calls.
- Held-out: baseline `p0` and the selected prompt x 24 cases = 48 calls.
- Maximum: 96 scored calls; if `p0` wins development, only its 24 held-out calls
  are needed, for a maximum of 72 calls in that branch.

The output is bounded by the response contract. Input tokens, output tokens, and
latency are recorded per call. Subscription access is not assigned a fabricated
USD price; any later monetary cost requires an explicit pricing basis.

## 6. Metrics and development selection

Operational validity is reported separately from semantic prediction. For
semantic metrics, an operational failure supplies no predicted state and counts
as a miss for the gold class; it is never silently dropped from a denominator.

Primary state metrics are oracle-aligned. Within each polarity, predicted and
oracle spans receive a deterministic maximum-cardinality one-to-one matching;
a predicted span aligns only when it contains the corresponding minimal oracle
span. Every emitted span must align or the case enters a distinct
`semantic_unaligned` column and supplies no primary predicted state. Empty
`neither` is vacuously aligned. Raw presence-state metrics are reported
separately. Span precision/recall/F1 and exact evidence-set accuracy are also
reported; exact evidence-set accuracy is diagnostic only.

This conservative engineering rule can false-negative a valid shorter
alternative or evidence absent from a non-exhaustive oracle. Repeated text can
also admit more than one equivalent byte interval. A later human-adjudicated
corpus must represent alternatives with explicit equivalence classes or resolve
these cases through blinded adjudication rather than adding flat oracle spans;
the metric is not a general proof of semantic correctness.

The primary metric is macro-F1 over the four oracle-aligned states. A development
prompt is eligible only if:

1. its response-contract rate is no worse than `p0`;
2. at least 9 of the 12 gold decisive cases (`support_only` or `refute_only`) are
   predicted decisively and correctly; and
3. recall is at least 3 of 6 for each of the four states.

These gates prevent an all-`neither` or generally over-abstaining prompt from
winning through low selective risk. Among eligible prompts, choose the highest
four-state macro-F1. A difference smaller than 0.02 is a tie; the tied prompt
with fewer mean output tokens wins, followed by the shorter prompt.

Every run also reports:

- exact four-state accuracy and the complete confusion matrix;
- precision, recall, and F1 for support presence and refutation presence;
- aligned decisive coverage and risk among aligned decisive predictions;
- decisive-gold recall;
- `both` conflict recall and `neither` recall;
- response-contract rate, exact-membership rate conditional on that contract,
  and fully mechanically valid rate;
- operational-error rate by error class;
- input tokens, output tokens, and latency; and
- all semantic metrics by clear/boundary and domain-family strata.

Selective risk is never reported without its coverage and decisive-gold recall.

## 7. Held-out comparison

After development selection, freeze the selected prompt and its digest. Run it
and `p0` on held-out inputs in interleaved randomized order under the same model,
information, call count, and execution settings.

For pilot adoption, the selected prompt must satisfy all of these held-out rules:

- four-state macro-F1 is not below `p0`;
- decisive-gold recall is at least 9 of 12 and no more than one case below `p0`;
- response-contract rate is no worse than `p0`; and
- no clear/boundary or domain stratum hides a material regression in the full
  reported table.

For this pilot, a material stratum regression means at least two fewer exact
four-state matches than `p0` within any 12-case clarity or domain-family stratum.

If it fails, retain `p0` and record a negative result. The held-out results may
not be used to modify a prompt and rerun the same oracle. Further development
requires a freshly constructed and committed held-out round.

## 8. Hold-out and anti-overfitting rules

Before prompt development, freeze the split manifest, annotation contract,
response semantics, metric implementation, eligibility gates, and run budget.

Held-out input wording, oracle bytes, and a fresh random nonce remain outside
the prompt-author-visible checkout until the selected prompt, configuration,
and rationale are committed and pushed. The checkout contains only a commitment
to their canonical bytes. The heldout run verifies and snapshots that commitment
before any model call.
After held-out predictions and run manifests are digested, disclose the oracle
and nonce and verify the commitment over the exact bytes used for scoring. The
completed portable run bundle—including manifest, jobs, predictions, raw index,
and snapshot digests—must be committed and pushed to the private remote before
disclosure. That external commit is the pre-reveal receipt; a locally rewritable
completion file alone is not a seal.

The round order is strict:

1. withhold heldout wording, oracle, and nonce;
2. run development and select `p0` or `p1`;
3. commit and push the prompt, configuration, and selection rationale;
4. disclose only the committed heldout inputs to the runner, verify their
   commitment, and snapshot them;
5. execute the heldout scored-first attempts;
6. commit and push the complete run bundle as the pre-reveal receipt; and
7. reveal the nonce and oracle, verify the bound commitment, and score once.

Runner modes `development`, `heldout`, and `engineering-smoke` enforce call
shape only. They make no claim that a corpus is human-adjudicated or otherwise
valid. Development accepts 24 cases and one or two prompts (`p0` and optional
`p1`); heldout requires
24 cases, one or two prompts, and prompt ID `p0`; engineering smoke permits one
prompt and at most four cases.

The following invalidate the round:

- exposing a held-out annotation, label, oracle path, or oracle-derived summary
  to the prompt author or assessor before predictions are frozen;
- exposing held-out input wording or domain families to the prompt author before
  the selected prompt, configuration, and rationale are frozen and pushed;
- moving a group or case between splits after seeing a model result;
- testing more than two prompts, changing models, or changing the scoring rule
  on the same held-out set;
- replacing a failed attempt with a retry or selecting the best of repeated
  attempts;
- omitting an operational failure from metrics; or
- suppressing an attempted prompt or an unfavorable result.

Case order, prompt order, and the held-out `p0`/winner interleaving are derived
from recorded seeds. Equal case content, prompt content, and run configuration
have stable digests in every result record.

## 9. Selta integration boundary

The opt-in evidence host seam now exists in the core and optional RPC `assess` lane
([../../docs/24-presence-assessment.md](../../docs/24-presence-assessment.md)). It does
not alter the binary protocol-1 `verify` envelope. A cautious integration gives the
same response this direct interpretation:

- a non-empty `support` array contributes support presence;
- a non-empty `refute` array contributes refutation presence;
- two empty arrays are semantic `neither`; and
- process, transport, parsing, schema, or quotation failures remain operational
  errors.

The real-host integration test must invoke an actual Codex child process and
exercise transport, response admission, accounting, and operational separation.
The 48-case experiment, rather than an ordinary deterministic unit test, carries
the semantic performance measurements.
