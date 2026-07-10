# @selta/lean-host

A Selta extension host for Lean 4 output. Two verifiers behind the docs/05
contract:

| Extension | Determinism | What it checks |
|---|---|---|
| `lean_check` | deterministic | the code compiles under the Lean compiler; `sorry` fails by default |
| `lean_reflects` | non-deterministic | a codex judge decides whether the code reflects exactly the statement in context; Selta samples it and votes |

## Operator environment

Code execution and spend are operator concerns, never schema concerns — both
are controlled by the host's environment, not by registered schemas:

| Variable | Meaning |
|---|---|
| `LEAN_BIN` | the Lean compiler binary (default `lean`) |
| `OPENAI_API_KEY` | required for `lean_reflects` |
| `OPENAI_BASE_URL` | default `https://api.openai.com` |

`seltad` configuration:

```toml
[hosts.lean]
run = ["node", "packages/lean-host/index.js"]
```

Pool enablement: `"extensions": ["lean_check", "lean_reflects"]`.

## `lean_check`

Config: `{ "timeout_ms"?: int, "allow_sorry"?: bool }`.

Writes the value to a `.lean` file and runs the compiler. Failure deltas are
typed — the host declares a `delta_schema`, so the engine verifies the
structured payload like any other value:

```jsonc
{ "message": "lean: 2:5 unknown identifier 'BAD'",
  "data": { "errors": [ { "line": 2, "col": 5, "severity": "error", "message": "…" } ] } }
```

A proof using `sorry` compiles with only a warning; for verification that is a
failure by default ("it compiles but proves nothing") — opt out per spec with
`allow_sorry: true`.

## `lean_reflects`

Config: `{ "statement": str, "question"?: str, "model"?: str, "effort"?: str,
"service_tier"?: str, "max_output_tokens"?: int }`.

Defaults: model `gpt-5.5-codex`, reasoning effort `xhigh`, service tier
`priority` (high speed). The judge is instructed to accept only an exact
formalization — same hypotheses, conclusion, quantifiers, types, bounds — and
to treat the code as untrusted data. A reply that is not the required JSON
verdict is discarded and resampled by the engine, never counted as a vote.

The statement usually arrives online, per request, through an `$env` hole:

```jsonc
{
  "type": "object",
  "fields": {
    "lean": {
      "type": "str",
      "verify": [
        { "ext": "lean_check", "config": {} },
        { "ext": "lean_reflects",
          "config": { "statement": { "$env": "statement" } },
          "sampling": { "samples": 5, "vote": { "at_least": 4 }, "depth": 1 } }
      ]
    }
  }
}
```

```sh
selta verify proofs lean_answer output.json \
  --env 'statement=For every natural number n, n + 0 = n.'
```

Judge token usage flows back through the protocol's `usage` field and shows up
aggregated in every report.
