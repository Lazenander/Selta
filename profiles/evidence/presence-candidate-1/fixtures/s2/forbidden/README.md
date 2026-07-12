# Presence S2 forbidden mutations

| Mutation | Nonconforming behavior exposed |
|---|---|
| `two-support.collapsed-basis.outcome-package.json` | Drops one distinct support provenance ID while retaining the state name |
| `support-only.omitted-closure-assumption.outcome-package.json` | Uses an assumption but omits it from state and conclusion closures |
| `both.to-neither-state.outcome-package.json` | Erases explicit conflict as if it were absence |
| `neither.absence-to-support.outcome-package.json` | Creates support from absence without an evidence node or typed rule |

All four mutations preserve package and typed-value shape. Conformance is the
exact semantic distinction from their named positive oracle, not merely Selta
shape acceptance.
