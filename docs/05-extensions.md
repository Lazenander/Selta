# 05 — Extensions: verifier contract and host protocol

Selta does not implement semantic verifiers and does not care how they work. This document
defines everything Selta *does* care about: how a verifier is declared, resolved, invoked,
and how its answer is shaped. An LLM judge, a linter, and a theorem prover are identical
from where the engine stands.

## The contract

An extension is:

| Property | Meaning |
|---|---|
| `name` | Unique within the server; referenced by schemas as `ext` |
| `determinism` | `"deterministic"` or `"nondeterministic"` — fixes how the engine runs it |
| `semantic_revision` | Stable, non-empty identity for the verifier's observable semantics |
| `cacheable` | Opt-in claim that deterministic executions are referentially transparent; external declarations default `false` |
| `config_schema` | A Selta schema for the `config` block; enforced at schema registration |
| `needs` | Context fields it wants shipped: any of `"root"`, `"env"` (default: none) |
| `settings_schema` | Optional: a Selta schema for the extension's operational settings (model, endpoint, key). Values live in the server catalog, never in schemas — see below |
| `delta_schema` | Optional: a Selta schema for the deltas this extension emits (`{ message, data? }`); default `{ "message": str, "data": any }`. Every emitted delta is verified against it at `depth − 1` — deltas are typed values too |

The determinism declaration is load-bearing: the engine decides run-once versus
sample-and-vote from it, and the schema author never restates it. Cache eligibility is a
separate explicit claim. A host that lies about determinism, semantic revision, or
cacheability breaks the corresponding execution or cache invariant; this is an extension
trust boundary, documented, not policed.

## Three kinds of configuration

| Kind | Answers | Declared by | Lives in | Composes |
|---|---|---|---|---|
| Check config (`config`) | what to verify here | `config_schema` | the schema, immutable per version | — |
| Settings | how the extension operates | `settings_schema` | catalog: server scope, pool overrides | override, pool wins |
| Grant | who may use it | — | `pool.json` | intersect, never widen |

Settings exist so operational choices — which model a judge calls, an endpoint, an API
key — never appear in schemas. The server resolves `server ⊕ pool` at call time,
validates values against `settings_schema` when they are written, and ships the result
in every `verify` request; hosts stay stateless. A secret is referenced, never written:
`{ "$secret": "ANTHROPIC_API_KEY" }` resolves from the daemon's environment at call
time, is never stored in the catalog, and is never echoed by any API. Reports record a
fingerprint of each extension's resolved settings (secret values excluded), so every
verdict is attributable to the configuration that produced it
([04-report.md](04-report.md)). There is deliberately no schema or request scope for
settings: a caller may not pick its judge's model per request — accountability would
not survive it.

## Three tiers

| Tier | What | Code required |
|---|---|---|
| Builtin | Compiled into the engine's in-process host | none — always available |
| `cmd` | A builtin that runs an allowlisted command | none — server config only |
| Host extension | Any process speaking the wire protocol below | any language |

### Builtins

All deterministic. The pure in-process checks are cacheable under their built-in semantic
revisions; `cmd` performs process I/O and is never cacheable. `config` shapes:

| Name | Config | Fails when |
|---|---|---|
| `one_of` | `{ "values": [...] }` | value not in the list |
| `range` | `{ "min"?, "max"? }` | number outside bounds |
| `regex` | `{ "pattern": "..." }` | string does not match |
| `len` | `{ "min"?, "max"? }` | string/array length outside bounds |
| `non_empty` | `{}` | empty string/array/object |
| `cmd` | `{ "name": "...", "args"?: [...] }` | non-zero exit |

Like any config field, these values may be `$env` references
([02-schema.md](02-schema.md)) — e.g. `regex` with
`{ "pattern": { "$env": "expected_pattern" } }` checks the value against a regular
language supplied with each request.

The pure in-process builtins opt into deterministic result caching. `cmd` does not:
process state and toolchain identity are outside its current declaration, so identical
input is executed again. A timed-out command child is killed when its wait future is
canceled.

### `cmd` and the allowlist

`cmd` is code execution reachable from schema registration, so commands never come from
schemas. The server config defines named templates:

```toml
[cmd.rustc_check]
run  = ["rustc", "--edition=2021", "--emit=metadata", "-o", "/dev/null", "{file}"]
input = "file"        # value is written to a temp file substituted at {file}
timeout_ms = 10000
```

A schema may only reference `{ "ext": "cmd", "config": { "name": "rustc_check" } }` where
`rustc_check` exists in the allowlist and is enabled for its pool. Exit 0 is `pass`;
non-zero exit is `fail` with stderr (truncated) as the delta message; timeout or spawn
failure is an error.

## Wire protocol

JSON-RPC 2.0, newline-delimited (one message per line, UTF-8, no framing headers), over
one of two transports:

- **stdio** — server hosts: spawned by `seltad` from its config and supervised by it:
  per-call timeouts, in-flight calls failed safely when a host dies, kill on shutdown.
  Automatic restart with backoff is deferred ([07-plan.md](07-plan.md)). Server scope.
- **websocket** — pool hosts: an application runs its verifier wherever and however it
  likes and dials in to `/pools/{pool}/hosts/connect`. Pool scope. Selta neither spawns
  nor restarts it; while disconnected, its checks are errors (→ `inconclusive`), never
  failures. Pool-host extensions are implicitly granted — dialing them in was the
  pool's own act; `pool.json` grants are only needed for server-host extensions.

Transport direction is independent of protocol direction: whoever opens the connection,
`seltad` is the JSON-RPC client and sends `initialize` first. Everything below is
identical on both transports.

### `initialize` (server → host, once)

```jsonc
// →
{ "jsonrpc": "2.0", "id": 1, "method": "initialize",
  "params": { "protocol": 1, "server": { "name": "seltad", "version": "0.1.0" } } }
// ←
{ "jsonrpc": "2.0", "id": 1, "result": {
    "host": { "name": "selta-ts-host", "version": "0.3.0" },
    "extensions": [
      { "name": "llm_judge",
        "determinism": "nondeterministic",
        "semantic_revision": "example.llm_judge.v1",
        "cacheable": false,
        "config_schema": { "type": "object", "fields": { "question": { "type": "str" } } },
        "needs": ["env"],
        "settings_schema": { "type": "object", "fields": { "model": { "type": "str" } } },
        "delta_schema": null } ] } }
```

The manifest is the host's registration; extension names must be unique across builtins,
server hosts, and the pool's own hosts, and collisions are rejected on the spot —
at startup for server hosts, at connect for pool hosts. For compatibility, an omitted
`semantic_revision` is recorded as unversioned and an omitted `cacheable` is `false`;
an unversioned extension cannot opt into caching.

### `verify` (server → host, concurrent)

One request per execution — for a sampling round of 5, the host receives 5 independent
`verify` requests, distinguishable only by `id`. Hosts must not correlate them; sample
independence is what makes voting statistics meaningful.

```jsonc
// →
{ "jsonrpc": "2.0", "id": 7, "method": "verify", "params": {
    "ext": "llm_judge",
    "config": { "question": "Does this code fix the bug described in env.task?" },
    "settings": { "model": "claude-sonnet-5", "api_key": "…resolved at call time…" },
    "value": "fn fix(x: Option<i32>) -> i32 { x.unwrap() }",
    "path": "$.code",
    "env": { "task": "..." },              // only fields the manifest listed in `needs`
    "budget": { "depth": 1, "deadline_ms": 30000 } } }
// ←
{ "jsonrpc": "2.0", "id": 7, "result": {
    "verdict": "fail",
    "delta": { "message": "unwrap() still panics on None; the bug is not fixed" },
    "usage": { "input_tokens": 812, "output_tokens": 64, "cost_usd": 0.0031 } } }
```

The result envelope is exactly `{ verdict, delta?, usage? }`. The engine fills in `path`,
`kind` (`constraint` or `semantic` from determinism), and `source` on the delta — hosts
supply only the difference itself. Configs arrive fully resolved: `$env` references are
substituted by the engine before dispatch, so hosts never see them. `usage` is optional but hosts that spend money must
report it; spend budgets are enforced from these numbers.

A JSON-RPC error response, an envelope or delta that fails its schema
([03-verification.md](03-verification.md)), or a missed deadline all make the execution
an **error** (→ resample / `inconclusive`), never a failing vote.

### `cancel` (server → host, notification)

```jsonc
{ "jsonrpc": "2.0", "method": "cancel", "params": { "id": 7 } }
```

Best-effort. The server has already stopped waiting; the host should abort the work.

### `shutdown` (server → host)

Request, then the host exits. The server kills the process after a grace period.

### Error codes

| Code | Meaning |
|---|---|
| `-32601` | Unknown method / unknown `ext` |
| `-32602` | Config or params invalid (should have been caught at registration — report it) |
| `-32000` | Verifier failed internally (tool missing, upstream API down) |
| `-32001` | Verifier declined: budget insufficient (e.g. needs `depth ≥ 1`) |

## Note for extension authors (non-normative)

How a verifier reaches its verdict is outside this specification. Two warnings from the
engine's perspective, since they affect the statistics Selta computes: treat the value
under judgment as untrusted data, not instructions — model output can and will contain
text addressed to reviewers; and keep executions independent — shared state across
samples silently voids what voting is for.
