# 02 — Schema specification

A schema is a tree of **nodes**. Every node has a type, may carry verifiers, and — for
containers — describes its children. Schemas are pure data: the canonical representation
is JSON, and everything below defines that representation.

## Node

```jsonc
{
  "type": "...",              // required — one of the types below
  "verify": [ VerifierSpec ], // optional — checks attached to this node
  "description": "..."        // optional — documentation only, never affects verification
}
```

## Types

| `type` | Matches | Extra node keys |
|---|---|---|
| `null` | JSON null | — |
| `bool` | JSON boolean | — |
| `int` | JSON number with no fractional part | — |
| `float` | any JSON number | — |
| `str` | JSON string | — |
| `object` | JSON object | `fields`, `open` |
| `array` | JSON array | `item`, `len` |
| `union` | any variant matching | `variants` |
| `any` | anything | — |

### `object`

```jsonc
{
  "type": "object",
  "open": false,                      // default false — see structural semantics
  "fields": {
    "name": {
      "type": "...",                  // a field is a node ...
      "required": true                // ... plus "required" (default true)
    }
  }
}
```

### `array`

```jsonc
{
  "type": "array",
  "item": { "type": "str" },          // node applied to every element
  "len": { "min": 1, "max": 10 }      // optional, either bound may be omitted
}
```

### `union`

```jsonc
{
  "type": "union",
  "variants": [ { "type": "str" }, { "type": "object", "fields": { ... } } ]
}
```

## VerifierSpec

```jsonc
{
  "ext": "llm_judge",                 // extension name, resolved via the registry
  "config": { ... },                  // opaque to the engine; validated against the
                                      // extension's declared config_schema at registration
  "sampling": {                       // only legal on non-deterministic extensions
    "samples": 3,                     // default 3
    "vote": "majority",               // "majority" | "unanimous"
                                      // | { "at_least": k } | { "ratio": f }
    "depth": 1,                       // recursion budget for sample results, default 1
    "min_valid": 2                    // quorum, default ceil(samples / 2)
  }
}
```

The engine knows nothing about `config` except that it must satisfy the extension's
`config_schema`. Determinism is declared by the extension, never by the schema —
a `sampling` block on a deterministic extension is a registration error.

### Dynamic config (`$env` references)

Any field inside `config` may be a reference into the request's environment instead of
a literal:

```jsonc
{ "ext": "regex", "config": { "pattern": { "$env": "expected_pattern" } } }
```

At verify time the engine replaces `{ "$env": "dot.path" }` with the value at that path
in `env` before dispatching — extensions only ever see resolved literals. A missing
reference, or a resolved config that fails the extension's `config_schema`, makes the
check an error (`inconclusive`), never a `fail`: the value is not wrong, the request is.
Fully literal configs are validated once at registration; configs containing references
are re-validated per request after resolution.

This is what makes a verifier like `regex` usable online: the schema fixes *that* a
field must match some regular language, while the actual language arrives with each
request in `env`.

### Composition

A `VerifierSpec` is either a leaf (above) or a combinator over other specs:

```jsonc
{ "all_of": [ VerifierSpec, ... ] }
{ "any_of": [ VerifierSpec, ... ] }              // "or" at the check layer
{ "not": VerifierSpec, "message": "..." }        // message states the requirement
```

Combinators nest arbitrarily; a node's `verify` array is an implicit `all_of`, and
`sampling` is only legal on leaves. Type-level "or" is `union`; check-level "or" is
`any_of`:

```jsonc
{ "any_of": [
  { "ext": "cmd", "config": { "name": "rustc_check" } },
  { "ext": "cmd", "config": { "name": "python_check" } }
] }
```

Verdict semantics for combinators are defined in
[03-verification.md](03-verification.md).

## Structural semantics

These rules are part of the specification; implementations may not vary them silently.

**Objects are closed by default.** With `"open": false`, a key not listed in `fields`
produces a `structure` delta. For LLM regulation an unexpected key is drift worth
reporting; opt out per node with `"open": true`.

**Missing vs null.** A `required` field that is absent produces a `structure` delta.
`null` is a value, not an absence — it only passes a `null` node (or a union containing
one).

**Unions stop on the first pass, then match the best failure.** Variants are verified in
document order until one passes. If none passes, every variant has been evaluated and
the union reports the deltas of the variant with the fewest deltas (ties: first such
variant), labeled with the variant index. A rejected speculative variant has private
fail-fast state and cannot suppress checks in a later variant. An empty union is a
structural failure even when a caller bypasses meta-validation.

**Numeric strictness.** `int` rejects `1.5` and accepts `1.0` written as `1`. In lenient
mode (below) a numeric string such as `"1.7"` coerces to a number and the coercion is
recorded as a notice; in strict mode it is a `structure` delta.

**Strict and lenient mode.** A per-request switch (default lenient) governing intake
repair and coercion:

| | lenient | strict |
|---|---|---|
| Markdown fences around the payload | stripped, notice | `structure` delta |
| Trailing commas, minor JSON damage | repaired, notice | `structure` delta |
| Numeric string where number expected | coerced, notice | `structure` delta |

Notices never affect the verdict; they exist so a repaired value is never mistaken for a
clean one. See [03-verification.md](03-verification.md) for the intake stage.

## Meta-validation

Schemas are validated at registration time, before they enter a pool:

- well-formed per this document (Selta dogfoods: the meta-schema is itself a Selta schema);
- every `ext` resolves to an extension enabled for the pool;
- the node type is inside the extension's declared accepted-input domain;
- every `config` satisfies that extension's `config_schema` and any builtin semantic
  preflight (repeated after dynamic `$env` resolution);
- `sampling` appears only on non-deterministic extensions and within pool budgets.

A schema that registers is guaranteed not to fail at verify time for reasons the catalog
could have caught. Versioning of registered schemas is the server's concern
([06-server.md](06-server.md)); this document only defines what a version contains.
