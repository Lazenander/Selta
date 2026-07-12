# Held-out engineering result

Frozen 2026-07-12. This is a result over a balanced, author-constructed
engineering fixture. It is not the three-human-annotated formal pilot in
`PROTOCOL.md`, and it cannot establish semantic accuracy, prompt quality,
calibration, or deployment prevalence.

`p0` was the only prompt authorized by the development decision. There is no
`p1`, no held-out prompt comparison, and no held-out result may revise `p0`.

## Freeze and reveal chain

| Event | Commit |
|---|---|
| Development run recorded | `c876adc64f4f5d2b2ed64e87928ddc0ad3f237a6` |
| `p0` selection and held-out configuration frozen | `df4ae4e138fa0a143bf2efaa578d71f98fd58f2a` |
| Complete prediction bundle pushed before oracle disclosure | `ca50f909e6a8b1bdd1372ed1af41760a374ac991` |
| Optional authored-state compatibility recorded | `0df6d393104f0d3f4854fde2d3421fdce9db6a38` |
| Clarity-reporting compatibility recorded | `b9420f916bbd8de9cc257047f3182bb880d2554e` |
| Result and revealed corpus published | `c821ef18459aa517f759113bd933d0cf65540850` |

The pre-reveal receipt contains the manifest, jobs, prompt and input snapshots,
predictions, raw artifacts, raw index, and completion record, but no oracle,
nonce, or metrics. It is present on
`origin/agent/epistemic-composition-plan`.

## Bound artifacts

| Artifact | SHA-256 |
|---|---|
| `p0` (88 words) | `713d3e36a56d73f5584394f728b6539b6dc7800655935c9fd1f7f29524865eea` |
| Input JSONL, 24 records and 6,903 bytes | `e4290fbf27ad1ceb0e91a2295bede0995cafcfc87298fedb60539c44b31c3e25` |
| Commitment file | `331a52c7f6dad4b1b37a039558b52fad08201a07a20b6ccafd9d3a45bd630ccd` |
| Nonce-bound oracle commitment | `96f982eb98d98dfa09ad6756cf1e6725b3023cde057265bc25173cba1767c139` |
| Disclosed oracle, 24 records and 4,171 bytes | `14f59e4f2add8d374db2271ae7460cf6cc9807105a5cc5a6131912addc13cdb5` |
| Disclosed nonce file, 65 bytes; 32 decoded bytes | `235f5ceabb3879e66e7d9326d98f42680fccb69ba930c867e7207e35053fb798` |
| Codex transport schema | `4ba2eabf87e7a198055cba3dc57e5510ed3e0b4cbbe54f91932236318c4f4a1d` |
| Selta response schema | `039afc818f9fbe40490c127115b82d51996b51b6ac05a8aab100751ab538c120` |
| Manifest | `9e0dbb1ea7633d77b3fd2e207efd7276b6ffcd34c5c19ef3ec0a1275c71e5183` |
| Jobs | `f695041fc6ab44ef599d05066294f6d6c9dc508990b9764adbeae016ae950473` |
| Predictions | `3637098d833fb227641feafc56a494d64ec8da8316931ab9d681db96565d7c65` |
| Raw index | `4682d47393e7485ab140a131d411d9898904f90a18d9053a58ed7e7351dbd7a8` |
| Completion record | `56236cb59a091841810ec44db468c4364f04660085546d524e56d06417b8e345` |
| Final metrics | `c4c16f429236ab591847d1adc0dbf0adc0c152f2bd5b96c3bc313cc59987dcf4` |
| Held-out generation runner executable | `cd06442050ee3c8a7d4c93827459a9a657bc0f064e158f45796f42aabacb0f5d` |
| Codex program | `134063e133f0b4244fa3b251acf973d4fe4b4aeeacbdc135211bf480f59f1477` |
| Neutral harness instruction | `7fd9b89bc496883c3d87ffc1924454ebb7bc3eca64ecc7faefc34df101aee4aa` |

`raw-index.json` is the authoritative byte length and digest inventory for all
72 raw event, response, and stderr files. The metrics file binds the exact
input, manifest, jobs, predictions, raw index, completion record, prompt, and
disclosed-oracle digests above.

## Execution and verification

The run used Codex CLI `0.144.1`, `gpt-5.6-terra`, low reasoning, seed
`2026071202`, concurrency `4`, and a `180` second per-call timeout. It contains
24 scored first attempts and no retries. The manifest records the complete
optional-capability denylist, neutral configuration, child command template,
and isolation policy.

From this experiment directory, the frozen run arguments were:

```sh
cargo run --locked --offline --manifest-path runner/Cargo.toml -- run \
  --mode heldout \
  --inputs /private/tmp/selta-real-codex-holdout.inputs.jsonl \
  --prompt p0=prompts/p0.txt \
  --schema schemas/response.schema.json \
  --selta-schema schemas/response.selta.json \
  --output runs/heldout-p0-terra-low-001 \
  --model gpt-5.6-terra \
  --reasoning low \
  --seed 2026071202 \
  --concurrency 4 \
  --timeout-seconds 180 \
  --commitment corpus/holdout.commitment.json
```

The disclosed bytes verify with:

```sh
cargo run --locked --offline --manifest-path runner/Cargo.toml -- \
  verify-commitment \
  --commitment corpus/holdout.commitment.json \
  --inputs corpus/holdout.inputs.jsonl \
  --oracle corpus/holdout.oracle.jsonl \
  --nonce corpus/holdout.nonce
```

The bound scoring command is:

```sh
cargo run --locked --offline --manifest-path runner/Cargo.toml -- validate \
  --run runs/heldout-p0-terra-low-001 \
  --oracle corpus/holdout.oracle.jsonl \
  --nonce corpus/holdout.nonce \
  --output runs/heldout-p0-terra-low-001/metrics.json
```

## Compatibility record

The first post-reveal scoring attempt stopped before writing metrics with
`invalid oracle on line 1; missing field state` because the committed oracle
omitted the redundant authored `state` field. Commit
`0df6d393104f0d3f4854fde2d3421fdce9db6a38` makes wire `state` optional, always
derives canonical state from the unchanged support/refutation arrays, and
rejects a supplied state that disagrees.

The first successful metrics file then had an empty clarity table. The frozen
input omitted optional clarity metadata, while every disclosed oracle record
carried it. Commit `b9420f916bbd8de9cc257047f3182bb880d2554e` uses input
clarity when present and otherwise oracle clarity, rejecting disagreement. This
only restores required reporting strata. Neither correction changes a prompt,
prediction, corpus byte, state, alignment rule, or metric definition.

## Result

All 24 responses were Selta-admitted and mechanically valid. There were no
operational errors or tool events. Usage was 60,327 input tokens, 43,776 cached
input tokens, and 801 output tokens; mean recorded latency was 4,004.125 ms.

| Measure | Result |
|---|---:|
| Raw presence-state accuracy | `24/24` (`1.0000`) |
| Raw four-state macro-F1 | `1.0000` |
| Conservative aligned-state accuracy | `18/24` (`0.7500`) |
| Conservative aligned four-state macro-F1 | `0.8439` |
| Exact evidence-set accuracy | `18/24` (`0.7500`) |
| Aligned decisive-gold recall | `7/12` (`0.5833`) |
| Aligned decisive coverage / risk | `7/24` (`0.2917`) / `0.0000` |
| `support_only` / `refute_only` recall | `4/6` / `3/6` |
| `both` / `neither` recall | `5/6` / `6/6` |
| Support-span precision / recall / F1 | `0.7692` / `0.8333` / `0.8000` |
| Refutation-span precision / recall / F1 | `0.7857` / `0.9167` / `0.8462` |

Selective risk is reported only with its low coverage and decisive-gold recall.
The engineering result does not meet the preregistered `9/12` aligned
decisive-recall adoption gate.

### Strata

| Stratum | Cases | Raw state accuracy | Aligned accuracy | Aligned macro-F1 | Exact sets | Decisive recall |
|---|---:|---:|---:|---:|---:|---:|
| Clear | 12 | `1.0000` | `0.7500` | `0.8250` | `0.7500` | `3/6` |
| Boundary | 12 | `1.0000` | `0.7500` | `0.8500` | `0.7500` | `4/6` |
| Medical scheduling | 12 | `1.0000` | `0.5833` | `0.7000` | `0.5833` | `2/6` |
| Open-source governance | 12 | `1.0000` | `0.9167` | `0.9500` | `0.9167` | `5/6` |

The domain difference is large under canonical-span alignment, even though raw
presence state is exact in both domains. It must not be hidden by the pooled
score.

## Span audit and interpretation

The six exact-set mismatches are also the six `semantic_unaligned` cases, and
all six preserve the oracle's raw state:

- `hv1-003`, `hv1-009`, and `hv1-023` use shorter exact excerpts than the flat
  canonical oracle span, so the predicted span does not contain that full span;
- `hv1-007`, `hv1-017`, and `hv1-020` add a same-polarity supplementary excerpt
  absent from the flat oracle.

Those supplementary or shorter excerpts may be semantically acceptable, but
this author oracle has neither independent annotators nor equivalence classes
that can establish that. The gap between perfect raw state and conservative
alignment therefore demonstrates sensitivity to canonical-span representation;
it does not prove that the model is semantically perfect or that the scorer is
wrong.

## Decision and limitations

`p0` remains the selected baseline because development did not authorize a
candidate prompt, not because held-out data improved or validated it. This
round is closed: no prompt may be tuned and rerun against this oracle.

The fixture is synthetic, balanced rather than prevalence-weighted, small, and
written together with an author oracle. It lacks three blinded annotations,
retained disagreement, independent adjudication, and alternative-span
equivalence classes. Subscription use is not assigned a fabricated USD cost.
A formal pilot still requires the independent corpus and annotation procedure
in `PROTOCOL.md`.
