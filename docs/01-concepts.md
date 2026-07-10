# 01 — Concepts

## The three core ideas

1. **A schema is a JSON-shaped type tree.** It regulates which keys exist and what their
   values look like, recursively: objects contain fields, fields contain nodes, nodes may
   be objects again. This is the "type" part.

2. **Any node can carry verifiers.** A verifier is a single function:

   ```text
   verify(value, context) → pass | fail(delta) | error
   ```

   Verifiers are not part of Selta — they are extensions, referenced by name from the
   schema and resolved through a registry at runtime. Selta defines the contract and the
   orchestration; the implementation behind the contract is opaque. This is the
   "semantic" part.

3. **Every verifier declares itself deterministic or non-deterministic.**
   A deterministic verifier (regex, range, a compile check) gives the same verdict for the
   same input: run once, trust it. A non-deterministic verifier (an LLM judge) gives one
   *vote*, not the truth: the engine samples it N times, verifies each sample's result
   recursively with a reduced depth budget, and folds the votes with a voting policy.
   This is where "delta" earns its name — failing votes carry critiques, and the merged
   critiques are the delta.

## The boundary

```text
input:   schema (from the pool), value (model output), env (caller-supplied context)
output:  report — a verdict tree; every failure is a path-addressed delta
```

Selta never acts on a report. It does not regenerate answers, call models to fix code, or
rank candidates. It also never looks inside a verifier: a judge that runs three internal
prompts and a linter that shells out to `eslint` are indistinguishable to the engine —
both are an extension name, a determinism declaration, and a verdict.

## Worked example

An application registers this schema as `bugfix` in its pool (full syntax in
[02-schema.md](02-schema.md)):

```json
{
  "type": "object",
  "fields": {
    "language": {
      "type": "str",
      "verify": [ { "ext": "one_of", "config": { "values": ["rust", "python"] } } ]
    },
    "code": {
      "type": "str",
      "verify": [
        { "ext": "cmd", "config": { "name": "rustc_check" } },
        { "ext": "llm_judge",
          "config": { "question": "Does this code fix the bug described in env.task?" },
          "sampling": { "samples": 5, "vote": { "at_least": 4 }, "depth": 1 } }
      ]
    },
    "confidence": {
      "type": "float",
      "verify": [ { "ext": "range", "config": { "min": 0.0, "max": 1.0 } } ]
    },
    "tests": {
      "type": "array", "item": { "type": "str" }, "required": false,
      "verify": [ { "ext": "len", "config": { "min": 1 } } ]
    }
  }
}
```

The application submits a model output for verification:

```json
{
  "language": "rust",
  "code": "fn fix(x: Option<i32>) -> i32 { x.unwrap() }",
  "confidence": 1.7
}
```

Selta returns a report (full format in [04-report.md](04-report.md)); rendered:

```text
verdict: fail

✓  $.language     one_of
✓  $.code         cmd:rustc_check   compiles
✗  $.code         llm_judge         1/5 votes pass (need ≥ 4)
                                    "unwrap() still panics on None; the bug is not
                                     fixed — use unwrap_or or match"
✗  $.confidence   range             expected 0.0..=1.0, actual 1.7
```

What happened at `$.code`:

- `cmd:rustc_check` is deterministic — ran once, passed.
- `llm_judge` is non-deterministic — the engine requested 5 samples from the host.
  Each sample's envelope was verified structurally, and each failing sample's delta —
  itself a typed value — against the extension's `delta_schema`, before being counted
  as a vote. One malformed reply would have been an error, not a vote, and resampled
  within budget.
  4 of 5 valid votes said fail; the policy needed 4 passes; the failing critiques merged
  into one delta.

What Selta did **not** do: decide what to tell the model next, re-prompt anything, or fix
the code. The two ✗ deltas are the complete output. How the caller uses them is the
caller's business.

## Where things run

| Piece | Where | Defined in |
|---|---|---|
| Structural checks | In-process, built into the engine | [03-verification.md](03-verification.md) |
| Builtin verifiers (`regex`, `range`, `one_of`, `len`, `cmd`, ...) | In-process host | [05-extensions.md](05-extensions.md) |
| External verifiers (`llm_judge`, ...) | Host processes over JSON-RPC | [05-extensions.md](05-extensions.md) |
| App-provided verifiers | the app's own host, dialed in to its pool | [05-extensions.md](05-extensions.md) |
| Schema storage, pools, versioning | `seltad` catalog | [06-server.md](06-server.md) |
| Embedded use without a server | `selta-core` as a library | [07-plan.md](07-plan.md) |
