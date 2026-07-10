# Selta

**Se**(semantic) + d**elta** — a schema layer for regulating LLM output.

A Selta schema is a JSON-shaped type tree with verifiers attached to its nodes. Verification
takes a schema, a value (typically raw model output), and an environment, and returns a
**report** in which every failure is a **delta**: the precise difference between the answer
that was given and an answer that would be acceptable — a type mismatch, a violated
constraint, a compiler error, a judge's critique.

```text
(schema, value, env)  ──►  Selta  ──►  report { verdict, deltas }
```

That line is the entire contract. Everything on the left of the arrow is input; everything
Selta produces is on the right; nothing else crosses the boundary.

## Scope

Selta is a type-check layer and nothing more.

**In scope**

- The schema language: types, keys, nesting, recursion (`docs/02-schema.md`).
- The verification engine: structural checking, verifier orchestration — determinism,
  sampling, voting, recursion depth, budgets (`docs/03-verification.md`).
- The output contract: verdicts, deltas, reports (`docs/04-report.md`).
- The extension contract: how verifiers plug in, not how they work (`docs/05-extensions.md`).
- The server: one daemon, many applications, one schema pool per application, SQL-style
  (`docs/06-server.md`).

**Out of scope (deliberate non-goals)**

- What consumers do with a report. Retry loops, prompt rendering, repair strategies —
  Selta emits deltas and does not care what happens next.
- How verifiers are implemented. An LLM judge, a linter, a compiler wrapper are all opaque
  extensions behind one contract. Selta orchestrates them without knowing their internals.
- LLM provider clients, prompt engineering, application data storage.

## Documents

| Document | Contents |
|---|---|
| [docs/01-concepts.md](docs/01-concepts.md) | Core model, terminology, worked example |
| [docs/02-schema.md](docs/02-schema.md) | Schema specification: types and structural semantics |
| [docs/03-verification.md](docs/03-verification.md) | Engine semantics: pipeline, verdicts, voting, depth |
| [docs/04-report.md](docs/04-report.md) | Output contract: report and delta format |
| [docs/05-extensions.md](docs/05-extensions.md) | Verifier contract and host wire protocol |
| [docs/06-server.md](docs/06-server.md) | `seltad`: pools, catalog, versioning, HTTP API |
| [docs/07-plan.md](docs/07-plan.md) | Workspace layout, milestones, decision log |
| [docs/08-theory.md](docs/08-theory.md) | Theoretic model: refinements, K3, change structures, kernels, step-indexing |

## Glossary

| Term | Meaning |
|---|---|
| Schema | A type tree with verifiers attached; the unit an application registers |
| Node | One position in a schema tree (an object, an array, a string, ...) |
| Verifier | A check attached to a node: `(value, context) → verdict` |
| Extension | A named verifier implementation resolved through the registry |
| Host | A process serving extensions over the wire protocol: server-spawned (stdio) or app-connected to a pool (websocket) — plus the builtin in-process host |
| Settings | How an extension operates (model, endpoint, key); valued at server and pool scope, resolved per call |
| Verdict | `pass`, `fail`, or `inconclusive` |
| Delta | The difference between the given value and an acceptable one; attached to `fail` |
| Notice | Informational annotation that does not affect the verdict (e.g. an intake repair) |
| Sample | One execution of a non-deterministic verifier; yields one vote or one error |
| Vote | A valid sample result, aggregated by a vote policy |
| Depth | Remaining budget for recursively verifying verifier output — deltas are typed values too |
| Pool | A per-application namespace of schemas inside `seltad` (the analog of a database) |
| Report | The complete output of one verification: a verdict tree plus flattened deltas |

## Quick start

```sh
cargo build --workspace

./target/debug/seltad selta.toml &          # all-defaults config works too

./target/debug/selta pool create demo
cat > slug.schema.json <<'EOF'
{ "type": "str",
  "verify": [ { "ext": "regex", "config": { "pattern": { "$env": "expected_pattern" } } } ] }
EOF
./target/debug/selta schema put demo slug slug.schema.json
echo '"hello-world"' > value.json
./target/debug/selta verify demo slug value.json --env 'expected_pattern=^[a-z-]+$'
```

The schema fixes *that* the value must match some regular language; the actual language
arrives online, with each request, through `env` — the `$env` mechanism of
[docs/02-schema.md](docs/02-schema.md). For raw model output (fences and all), pass
`--text` and intake repairs what it can, leaving notices.

## Status

v0.1 implemented and verified end-to-end: `selta-core` (engine, builtins, combinators,
voting, depth recursion, cache, settings, monitoring), `selta-protocol` +
`@selta/extension` (TypeScript SDK), `seltad` (sqlite-backed catalog with a file-tree
option behind one storage trait, jobs, settings resolution with `$secret` references,
per-pool stats persisted across restarts, websocket pool hosts), and the `selta` CLI. The
docs/01 worked example runs over HTTP with a real compile check and a real Node judge
host; the docs/07 M4 acceptance — reconfiguring the judge at pool scope without touching
the schema — runs as an integration test, as does a dependency-free websocket pool host. A Lean 4 host ships in
[packages/lean-host](packages/lean-host/README.md): a compiler check with typed
structured deltas, plus a codex judge voting on whether the code reflects exactly the
statement supplied in context. Known deviations from the design live in the deferred
table of [docs/07-plan.md](docs/07-plan.md). Documents dated 2026-07-04.
