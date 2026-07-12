# Real-Codex vague-verifier experiment

Status: engineering preparation in progress. Engineering-only inputs, a development
oracle, and a committed holdout procedure exist; no formal three-human corpus or
completed model result exists.

This experiment asks a deliberately small question: can a concise, general
prompt make a real Codex assessor preserve textual support, textual refutation,
conflict, and absence without improving its apparent accuracy merely by
abstaining more often?

The assessor emits two collections of exact quotations:

```json
{"support": ["..."], "refute": ["..."]}
```

The four evaluation states are derived from collection presence. They are not a
second model-authored field:

| `support` | `refute` | Derived state |
|---|---|---|
| empty | empty | `neither` |
| non-empty | empty | `support_only` |
| empty | non-empty | `refute_only` |
| non-empty | non-empty | `both` |

## Current artifacts

- `PROTOCOL.md` freezes the pilot design, budget, metrics, and anti-overfitting
  rules.
- `prompts/p0.txt` is the zero-shot baseline prompt.
- `schemas/response.schema.json` is the minimal structured-output shape supplied
  to Codex. It is a transport guard, not the semantic authority.
- `schemas/response.selta.json` is the exact strict, pure-builtin Selta response
  contract.
- `corpus/dev.inputs.jsonl` and `corpus/dev.oracle.jsonl` are a balanced,
  author-constructed 24-case engineering fixture.
- `corpus/holdout.commitment.json` binds engineering heldout inputs and an
  oracle; input wording, oracle, and nonce remain outside the checkout until
  their protocol freeze points.
- `corpus/ENGINEERING-NOTICE.md` states the permitted uses and non-claims for
  these artifacts.
- [`runner/README.md`](runner/README.md) documents the Rust dry-run, frozen-run
  validation, raw audit, scoring, and commitment workflow. No real model result
  has completed.

The engineering artifacts are not the formal pilot corpus. A formal result still
requires newly sourced cases, three blinded human annotations per case, retained
disagreement, adjudication, and the group-separated development/held-out split in
`PROTOCOL.md`. No engineering score may be relabelled as that result.

## Scope boundary

This is empirical model evaluation, not a conformance suite. Existing finite
and presence-profile fixtures may later serve as deterministic aggregation and
projection controls, but they are not natural-language prompt-development
examples.

Protocol-1 `pass | fail` results cannot honestly encode `both` or `neither`. The
opt-in assessment contract in
[../../docs/24-presence-assessment.md](../../docs/24-presence-assessment.md) now
provides a direct four-state host seam while leaving legacy `verify` unchanged. A
Codex response must use that seam or remain an admitted value; it must not be
coerced into a legacy host verdict.
