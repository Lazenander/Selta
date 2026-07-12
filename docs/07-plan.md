# 07 — Implementation plan

## Workspace

```text
selta/
├── Cargo.toml                 # workspace
├── crates/
│   ├── selta-core/            # schema, engine, verdicts, report, registry, builtins
│   ├── selta-protocol/        # extension wire types (JSON-RPC envelopes), shared
│   ├── seltad/                # catalog, pools, scheduler, jobs, HTTP + SSE, host supervision
│   └── selta-cli/             # client: pools, schemas, verify, report rendering
└── packages/
    └── extension/             # @selta/extension — TS host SDK (convenience, non-normative)
```

### Dependency rules

- `selta-core` depends on nothing server-shaped: no HTTP, no filesystem catalog, no
  process spawning except the `cmd` builtin behind a trait. It is the embeddable, SQLite-like mode.
- `selta-protocol` is types only; both `selta-core` (host proxies) and any Rust host use it.
- `seltad` composes core + protocol; it owns every operational concern (persistence,
  scheduling, supervision, budgets-as-config).
- The TS SDK implements the protocol spec; nothing in Rust knows it exists.

### Core crate layout

```text
selta-core/src/
├── lib.rs             # verify(schema, value, env, opts) -> Report
├── schema.rs          # Node, Type, Field, VerifierSpec + JSON (de)serialization
├── meta.rs            # meta-validation (the Selta schema for Selta schemas)
├── intake.rs          # fence stripping, JSON repair, notices
├── engine.rs          # pipeline, fold rules, sampling, voting, depth, budgets
├── verdict.rs         # K3 Verdict plus four-state evidence presence, Delta, Notice, CheckError
├── report.rs          # Report, NodeResult, optional evidence summaries, ordering
├── registry.rs        # name -> host + extension; ExtensionHost trait
├── cache.rs           # content-hash cache trait, no-op default
└── host/
    ├── builtin.rs     # one_of, range, regex, len, non_empty, cmd
    └── rpc.rs         # RpcHost: ndjson JSON-RPC verify plus optional assess
```

Key crate choices: `serde`/`serde_json` (values are `serde_json::Value`), `indexmap`
(field order preserved), `tokio` (verification is I/O-bound; samples run concurrently),
`thiserror`. Keep the tree this small; additions need a reason.

## Milestones

Each milestone lands with the worked example from [01-concepts.md](01-concepts.md) as its
end-to-end test at that layer.

**M1 — structural core.** Schema parsing + meta-validation, intake, structural
verification, report format, builtin verifiers except `cmd`. Deterministic-only,
single-threaded, no hosts. *Done when: the worked example minus `cmd`/`llm_judge`
produces the specified report, byte-stable.*

**M2 — orchestration.** Verdict algebra with `inconclusive`, sampling, voting, quorum,
depth recursion over `result_schema`, budgets, cache trait, `cmd` behind the allowlist
trait. Non-determinism tested with a scripted in-process fake extension. *Done when:
vote/quorum/depth semantics in [03-verification.md](03-verification.md) are each pinned
by a test.*

**M3 — hosts.** `selta-protocol`, `RpcHost` with supervision (timeouts, restart,
cancel), the TS SDK, and a scripted TS host used in integration tests. *Done when: the
worked example runs end-to-end with `llm_judge` served by a real child process.*

**M4 — `seltad`.** File catalog, pools, versioning, scheduler with pool budgets, jobs,
HTTP + SSE API, settings resolution, websocket pool hosts, stats counters. *Done when:
the worked example runs over HTTP against a daemon — including once with `llm_judge`
reconfigured at pool scope without touching the schema — and budget exhaustion
demonstrably yields `inconclusive`, not `fail`.*

**M5 — CLI.** Pool/schema management, verify, job watching, report rendering.

## Deferred (hooks exist, nothing blocks on them)

| Item | Hook already in design |
|---|---|
| Spend caps per request and pool | hosts already report `usage`; enforcement pending |
| Per-check SSE streaming | jobs + SSE endpoint exist; needs an engine progress hook |
| Per-execution concurrency limits | v1 enforces per-job pool semaphores |
| Host auto-restart with backoff | `RpcHost` already fails in-flight calls safely on crash |
| Real cache store | `cache.rs` trait (an in-memory impl ships in `seltad`) |
| Report history + analytics (pass rates by version, cost by pool) | reports are serializable; storage trait |
| Schema fixture tests (`selta test`) | meta-validation + immutable versions |
| Friendlier authoring syntax | JSON stays the wire format regardless |
| Network auth (tokens per pool) | API shape unchanged; add middleware |

## Decision log

| Decision | Choice | Alternative rejected |
|---|---|---|
| Value representation | `serde_json::Value` | custom value enum — no benefit at this layer |
| Core concurrency | async (`tokio`) | sync — sampling five judges sequentially is dishonest architecture |
| Extension boundary | JSON-RPC 2.0, ndjson: stdio for server hosts, websocket dial-in for pool hosts | TS-specific server as the boundary; WASM (sandbox blocks the I/O verifiers exist for); dylib (no stable ABI) |
| Depth semantics | recursive: verifier output is typed — every delta is verified against the extension's `delta_schema` at `depth − 1`; non-deterministic checks skipped at depth 0 | depth as resampling rounds — weaker, loses "judge output is LLM output" |
| Check composition | `verify` is implicit `all_of`; `any_of` / `not` combinators, sampling at leaves only | "or" at the type level only — cannot express "compiles as Rust or as Python" |
| Dynamic config | config fields may be `{ "$env": "path" }` holes, resolved per request; missing → `inconclusive` | per-request schema forks, or smuggling dynamic parameters through the value |
| Errors vs deltas | three-valued verdicts; errors never vote, never delta | boolean pass/fail — conflates "wrong" with "unknown" |
| Objects | closed by default | open by default — unexpected keys are drift signals for LLM output |
| Unions | best-match (fewest deltas) | first-match — cheaper, much worse deltas |
| Intake | lenient by default, every repair a notice | strict by default — kills half of real model output at the parser |
| `cmd` security | server-side named allowlist only | command strings in schemas — RCE via schema registration |
| Scope | report is the boundary; verifier internals opaque | shipping retry/repair helpers — out of scope by definition of the layer |
| Extension settings | `settings_schema` in the manifest; values at server + pool scope; resolved per call; secrets by env reference | settings inside schemas — secrets in version control, ops changes forcing version bumps; request-scope overrides — unaccountable |
| App-provided verifiers | pool hosts dial in over websocket: same protocol, same manifest, implicitly granted to their pool | server spawning app code — RCE; a second webhook contract — two protocols to keep honest |
| Monitoring | counters at the contract boundary (calls, errors, latency, usage, vote agreement), aggregated in memory, flushed through the storage trait | a metrics pipeline — out of proportion for the layer |
| Catalog persistence | sqlite by default (rusqlite, bundled) behind a storage trait; files backend kept for diffability; a fresh sqlite catalog imports an existing file catalog once; one contract suite pins both backends | files only — counters lost on restart, no transactional versioning; an async db crate — dependency weight out of proportion for single-row reads and writes |
