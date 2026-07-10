# 04 — Report and delta format

The report is Selta's entire output and its product boundary. It must be complete enough
that a consumer can act on it without asking Selta anything else — and what the consumer
does with it (retry, rank, log, ignore) is explicitly not Selta's concern.

## Report

```jsonc
{
  "schema": "bugfix@3",             // pool-qualified name and pinned version
  "extensions": {                   // resolved-settings fingerprint per extension that ran
    "llm_judge": "fnv1a:9f2c…"      // secret values excluded from the hash
  },
  "verdict": "fail",                // fold at $ — "pass" | "fail" | "inconclusive"
  "deltas": [ Delta ],              // flattened, ordered (see Ordering)
  "notices": [ Notice ],            // informational, never affect the verdict
  "errors": [ CheckError ],         // what made checks inconclusive, if anything
  "root": NodeResult,               // full verdict tree for consumers that want structure
  "usage": {                        // aggregated from extension responses
    "samples": 5,
    "input_tokens": 3120,
    "output_tokens": 410,
    "cost_usd": 0.0142
  },
  "timing": { "started": "2026-07-04T12:00:00Z", "elapsed_ms": 8215 }
}
```

`deltas` and `root` carry the same information in two shapes: the flat list is what most
consumers want; the tree preserves where in the schema each result came from.
`extensions` makes every verdict attributable: together with the pinned schema version,
the settings fingerprints identify the exact configuration — model, endpoint — that
produced it ([05-extensions.md](05-extensions.md)).

## NodeResult

```jsonc
{
  "path": "$.code",
  "verdict": "fail",
  "checks": [                        // one entry per VerifierSpec on this node,
    {                                // plus one synthetic "structure" entry
      "source": "llm_judge",
      "verdict": "fail",
      "deltas": [ Delta ],           // non-empty iff verdict == "fail"
      "votes": { "pass": 1, "fail": 4, "errors": 0, "samples": 5 },  // non-deterministic only
      "skipped": false
    }
  ],
  "children": { "fields": { ... } } // objects: by field name; arrays: "items": [ ... ]
}
```

## Delta

The atom of the system. Every failure, from a missing key to a compiler error to a merged
judge critique, is exactly this shape:

```jsonc
{
  "path": "$.confidence",           // where in the value tree
  "kind": "constraint",             // "structure" | "constraint" | "semantic"
  "message": "expected a value in 0.0..=1.0, got 1.7",
  "expected": "0.0..=1.0",          // optional, machine-comparable when it exists
  "actual": "1.7",                  // optional
  "source": "range",               // extension (or "structure") that produced it
  "data": { ... },                  // optional, extension-typed payload
  "votes": { "pass": 1, "fail": 4, "errors": 0, "samples": 5 }   // optional
}
```

A delta is itself a typed value. The extension-authored part — `message` plus the
optional structured payload `data` — is regulated by the extension's declared
`delta_schema` ([05-extensions.md](05-extensions.md), default
`{ "message": str, "data": any }`) and verified like any other value at `depth − 1`
before the delta is accepted; a delta that fails its own schema makes its execution an
error, never a vote. The remaining fields on this page are Selta's and fixed.

| `kind` | Produced by | Example |
|---|---|---|
| `structure` | the engine's structural check (and strict-mode intake) | missing key, wrong type, unexpected key |
| `constraint` | deterministic verifiers | range violation, regex mismatch, non-zero exit |
| `semantic` | non-deterministic verifiers | "the bug is not fixed — unwrap() still panics" |

`message` is the normative field: it is the difference stated in words, and it must be
actionable without access to the verifier that produced it. `expected`/`actual` are
optional structured hints, present only when they are naturally machine-comparable.

## Notice

```jsonc
{ "path": "$", "message": "stripped markdown code fence", "source": "intake" }
```

Notices record repairs, coercions, and skipped checks (`depth exhausted`). They exist so
a repaired value is never mistaken for a clean one, and they never affect any verdict.

## CheckError

```jsonc
{ "path": "$.code", "source": "llm_judge", "error": "host timeout after 30000 ms", "samples_lost": 2 }
```

Errors explain `inconclusive`. They are disjoint from deltas by design: a delta says "the
value is wrong like this", an error says "Selta could not find out".

## Path syntax

JSONPath subset, unambiguous and cheap to parse:

```text
$                  root
$.key              object field
$.items[2]         array element
$["weird key"]     field whose name needs quoting
```

Union results are reported at the union node's own path; the chosen variant index appears
in the check entry (`"variant": 1`), not in the path.

## Ordering and stability

`deltas` is ordered by: `structure` first, then `constraint`, then `semantic`; ties broken
by document order of the path. The ordering, field names, and enum values on this page are
a stable contract — additive changes only. Anything a consumer might parse lives in this
document, and nothing here implies how a consumer should react to it.
