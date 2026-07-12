# Development decision

Frozen 2026-07-12 before any held-out input was restored or inspected.
This is an engineering decision over the author-constructed fixture, not a
formal semantic-performance claim.

## Run receipt

- Run: [`runs/dev-p0-terra-low-001`](runs/dev-p0-terra-low-001)
- Prompt: `p0`, 88 words, SHA-256
  `713d3e36a56d73f5584394f728b6539b6dc7800655935c9fd1f7f29524865eea`
- Codex: CLI `0.144.1`, `gpt-5.6-terra`, low reasoning
- Execution: seed `2026071201`, concurrency `4`, timeout `180` seconds
- Completion SHA-256:
  `147fbe09300e6d2546e319df5d25b1603ada8ae1b066aa7ae20662c63a00b0b4`
- Predictions SHA-256:
  `dc2877f7b91f573291a084f1d6e31eb6e3845b9ddc73585f4899c079a2b9456f`

All 24 first attempts were Selta-admitted and mechanically valid. There were no
operational errors or tool events. Total usage was 60,445 input tokens, 31,744
cached input tokens, and 933 output tokens.

## Development result

Raw evidence-presence state was correct for 22 of 24 cases: accuracy `0.9167`
and macro-F1 `0.9164`. The conservative canonical-span projection was correct
for 11 of 24 cases: accuracy `0.4583` and macro-F1 `0.6023`. Exact evidence sets
matched in 10 of 24 cases.

The 13 `semantic_unaligned` cases comprise nine service-integrity and four
software-reliability cases, split into seven clear and six boundary cases, but
11 retain the correct raw state. Their breakdown is six supplementary or
corroborating spans, four shorter alternatives to the oracle's canonical span,
and one case combining a shorter span with extra evidence. These expose the
known flat, non-exhaustive author-oracle limitation; they do not establish a
prompt error.

The remaining exact-set mismatch, `dev-sr-005`, preserves the aligned
`refute_only` state but selects only one of the oracle's two evidence units.

Only two cases change the raw state:

- `dev-sr-006` treats `Saving` and an unsaved-changes warning as support even
  though no `Saved` indicator occurred.
- `dev-si-011` treats a uniform public notice as refutation of a specific
  preferential-treatment offer.

Both are boundary cases and the mechanisms differ. Therefore they do not meet
the frozen trigger of at least four errors across both domains and clarity
strata with one general mechanism that is not mainly oracle disagreement.

The aligned eligibility gates also remain diagnostic failures: decisive-gold
recall is 4 of 12, and support-only, refute-only, and both recall are each 2 of
6; neither recall is 5 of 6. No candidate prompt is created to optimize around
the author oracle.

## Selection and held-out freeze

Retain `p0`; `p1` is not authorized, and there will be no development rerun.
The held-out run is frozen as:

- prompt: `p0` only, using the SHA-256 above;
- Codex CLI `0.144.1`, `gpt-5.6-terra`, low reasoning;
- seed `2026071202`, concurrency `4`, timeout `180` seconds;
- expected 24 scored first attempts; and
- output directory `runs/heldout-p0-terra-low-001`.

The withheld input commitment records 24 cases, 6,903 bytes, and SHA-256
`e4290fbf27ad1ceb0e91a2295bede0995cafcfc87298fedb60539c44b31c3e25`.
The nonce-bound oracle commitment is
`96f982eb98d98dfa09ad6756cf1e6725b3023cde057265bc25173cba1767c139`.
Input wording, oracle bytes, and nonce remain outside the checkout until this
decision is committed and pushed. The held-out CLI, configuration, schemas,
isolation policy, and prompt bytes must match development; the runner must
verify the input commitment before calls. The complete run bundle must then be
committed and pushed before oracle/nonce disclosure. No held-out result may
revise the prompt.
