# 06 — `seltad`: the server

One long-running daemon serves many applications, SQL-style: each application owns a
**schema pool** (the analog of a database), pools hold named, immutable, versioned
schemas (the analog of DDL), a verification request is the query, and the report is the
result set.

| SQL world | Selta world |
|---|---|
| database server | `seltad` |
| a database per app | a schema pool per app |
| `CREATE TABLE` | registering a schema version in a pool |
| executing a query | submitting a value for verification |
| result set | report |
| `CREATE EXTENSION` per database | extensions enabled per pool |
| `psql` | `selta` CLI |
| SQLite embedded mode | `selta-core` linked as a crate |

The last row is a real mode, not a metaphor: `seltad` is a shell around `selta-core`
adding the catalog, jobs, and multi-tenancy. Engine semantics are byte-identical embedded
and served.

## Catalog

Persisted state — pools, immutable schema versions, monitoring counters — lives behind
a storage trait with two backends, chosen by `storage` in `selta.toml`. One test suite
pins them to identical behavior; nothing above the trait knows which one is running.

- **`sqlite`** (default): one file, `data/selta.db` — transactional version assignment,
  WAL, durable counters. The sqlite library is bundled; the daemon stays
  dependency-free at deploy time. On first open, an existing file catalog at the same
  `data` path is imported once, so switching backends never loses pools.
- **`files`**: plain files, diffable and debuggable — the catalog reads as a tree:

```text
data/
└── pools/
    └── app_a/
        ├── pool.json               # grants, settings overrides, budgets
        └── schemas/
            └── bugfix/
                ├── 1.json          # immutable once written
                └── 2.json
```

`pool.json`:

```jsonc
{
  "name": "app_a",
  "extensions": ["llm_judge"],          // host extensions this pool may use
  "cmd": ["rustc_check"],              // allowlisted command templates this pool may use
  "settings": {
    "llm_judge": { "model": "claude-haiku-4-5" }   // pool-scope overrides, validated
  },                                                // against the settings_schema
  "budget": {
    "max_depth": 2,
    "max_samples_per_request": 16,
    "max_concurrency": 8
  }
}
```

**Versioning.** Registering `bugfix` again writes `bugfix/3.json`; nothing ever mutates
`bugfix/2.json`. Verify requests pin a version (`bugfix@2`) or take the latest
(`bugfix`); the report always records the resolved version. Immutable versions are the
reproducibility story: last week's outputs can be re-verified against exactly the schema
that judged them.

**Registration** runs meta-validation ([02-schema.md](02-schema.md)): well-formedness,
extension enablement, config validation, and per-leaf budget fit. For every
non-deterministic leaf, its explicit sampling depth/count (or Selta's defaults) must be
no larger than the pool caps. This is a feasibility check, not a reservation: the shared
request budget can still be exhausted by siblings, arrays, or retries. A schema that
registers cannot fail at verify time for catalog-detectable reasons.

Schema JSON stays as raw UTF-8 through admission. The daemon checks the closed grammar
and duplicate object keys before projecting it into the Rust model, persists a normalized
admitted source, and repeats strict admission whenever a version is fetched or executed.
Both storage backends preserve source bytes exactly. A damaged database row or a manual
edit to the file catalog therefore fails closed instead of being interpreted by the
compatibility parser.

The default registry exposes the pure in-process builtin profile. `cmd` appears only when
the daemon has at least one configured command template; a registry constructed without a
template provider cannot admit schemas that name it.

## Configuration resolution

One rule across the system, SQL-style: four scopes, most specific applies, one
composition mode per kind of setting.

```text
server (selta.toml)  →  pool (pool.json)  →  schema version  →  request
```

| Kind | Composes | Example |
|---|---|---|
| Budgets (`max_depth`, `max_samples`, concurrency) | min-clamp — lower scopes only tighten | pool caps depth at 2; schema asks 3 → gets 2 |
| Capabilities (extensions, `cmd` templates) | intersect — never widen | a schema cannot use what its pool was not granted |
| Extension settings | override — pool wins over server; stops at pool scope | pool pins `llm_judge` to a cheaper model |
| Options (`mode`, `fail_fast`, `wait_ms`) | override — most specific wins | a request switches to strict intake |

Extension settings stop at pool scope on purpose: which model a judge calls is an
operational and accounting decision, not a per-schema or per-request one
([05-extensions.md](05-extensions.md)).

## HTTP API

JSON over HTTP on a local unix socket (default) or TCP. V1 isolation is namespacing and
budgets; token auth arrives when it listens beyond localhost — correct single-machine
semantics before half-built security.

| Method and path | Purpose |
|---|---|
| `POST /pools` | Create a pool (body: `pool.json` shape) |
| `GET /pools` | List pools |
| `GET /pools/{pool}` | Pool config and schema listing |
| `PUT /pools/{pool}/schemas/{name}` | Register a schema → `{ "version": 3 }` |
| `GET /pools/{pool}/schemas/{name}@{v}` | Fetch a schema version |
| `POST /pools/{pool}/verify` | Submit a verification |
| `GET /pools/{pool}/jobs/{id}` | Report — partial while running, final when done |
| `GET /pools/{pool}/jobs/{id}/events` | SSE stream of results as nodes settle |
| `DELETE /pools/{pool}/jobs/{id}` | Cancel |
| `GET /extensions` | Server-scope extension manifests, for schema authors |
| `GET /pools/{pool}/extensions` | Enabled extensions: manifest, availability, resolved settings (secrets redacted) |
| `PUT /pools/{pool}/settings/{ext}` | Set pool-scope settings overrides |
| `GET /pools/{pool}/hosts/connect` | WebSocket upgrade: an app-provided pool host dials in |
| `GET /pools/{pool}/stats` | Per-extension counters (see Monitoring) |

A strict schema-admission rejection is `400` with stable, pointer-addressed issues:

```json
{
  "error": "schema rejected by strict admission",
  "issues": [
    { "code": "DUPLICATE_OBJECT_KEY", "pointer": "/type", "detail": "duplicate object key" }
  ]
}
```

Clients branch on `code` and use the RFC 6901 `pointer` to locate the problem; `detail`
is diagnostic prose. Rejected sources do not allocate a schema version. If persisted
bytes later fail the same check, read and verify endpoints return `500` with the typed
issues because the immutable catalog invariant has been violated.

### Verify

```jsonc
// POST /pools/app_a/verify — exactly one of "text" / "value"
{
  "schema": "bugfix@2",
  "text": "```json\n{ \"language\": \"rust\", ... }\n```",   // raw model output → intake
  // "value": { ... },                                      // already-parsed JSON
  "env": { "task": "fix the null-pointer bug in ..." },
  "options": { "mode": "lenient", "fail_fast": false, "wait_ms": 2000 }
}
```

Verification is asynchronous by nature — judge samples take seconds. The response is
`202 { "job": "..." }`, unless the request settles within `wait_ms`, in which case the
report returns directly with `200`. Deterministic-only schemas settle in milliseconds and
in practice always take the synchronous path.

The SSE stream ends with one final event: `report` (an [04-report.md](04-report.md)
payload) or `canceled`. Streaming per-check settling events is deferred
([07-plan.md](07-plan.md)); by design the stream never carries anything a poller could
not also see.

## Scheduling and budgets

Per pool, `max_concurrency` bounds concurrent verify jobs (per-execution limits are
deferred). Catalog admission rejects a non-deterministic leaf whose explicit or default
sampling request cannot fit the pool's `max_depth` and `max_samples`; request options are
then clamped to those pool caps before the engine sees them. The request-wide budget is
shared rather than pre-reserved per leaf, so exhaustion still makes affected checks
`inconclusive`, never `fail`, and never takes the daemon down. Spend caps are deferred —
the protocol already collects the `usage` numbers they need ([07-plan.md](07-plan.md)). Cancellation flows: HTTP
`DELETE` → job abort → in-flight host calls dropped, with `cancel` notifications on
per-call timeouts.

## Monitoring

Selta cannot see inside a verifier, so it monitors exactly what crosses the contract —
per pool × extension:

| Counter | Answers |
|---|---|
| calls, errors, timeouts | is it healthy? |
| latency p50 / p95 | is it slow? |
| usage totals (tokens, cost) | what does it spend? |
| votes pass / fail, agreement rate | is it decisive? |

Agreement — the fraction of valid samples that sided with each check's outcome — is the
one metric only Selta can compute, and it is the quality signal for a judge: an extension
that votes 3–2 every time is a coin, not a verifier. `GET /pools/{pool}/stats` returns
the counters; per-extension availability (server host running, pool host connected)
shows in `GET /pools/{pool}/extensions`. Counters aggregate in memory and are flushed
through the storage trait — every few seconds when dirty, and once more on graceful
shutdown — so they survive restarts. Per-report history and analytics remain deferred
([07-plan.md](07-plan.md)).

## Server configuration

```toml
# selta.toml
listen  = "unix:/var/run/selta.sock"
data    = "/var/lib/selta"
storage = "sqlite"                    # default; "files" for the diffable tree

[hosts.ts]
run = ["node", "/opt/selta/hosts/ts/main.js"]

[extensions.llm_judge.settings]
model   = "claude-sonnet-5"
api_key = { "$secret" = "ANTHROPIC_API_KEY" }   # from the daemon's environment

[cmd.rustc_check]
run = ["rustc", "--edition=2021", "--emit=metadata", "-o", "/dev/null", "{file}"]
input = "file"
timeout_ms = 10000
```

Hosts, command templates, and default settings are server-level resources; pools opt
in — and override settings — by name.

## CLI

`selta` is the `psql` of the system — a thin client over the HTTP API:

```text
selta pool create app_a
selta schema put app_a bugfix ./bugfix.schema.json      # → registered bugfix@3
selta verify app_a bugfix@3 ./output.json --env task="..."
selta job watch app_a 7f3a                              # streams settling checks
```

Report rendering in the CLI (the ✓/✗ view in [01-concepts.md](01-concepts.md)) is a
convenience of this one client, not part of the output contract.
