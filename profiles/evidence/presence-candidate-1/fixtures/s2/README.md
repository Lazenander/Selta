# Presence S2 law fixtures

These cases extend the immutable presence profile without adding a contract or
semantic descriptor. Every assessment uses the caller-supplied input root as
`ExpectedBasisRef`.

| Case | Required observation |
|---|---|
| `replay-equivalent` | Raw package bytes differ from `../neither.input-package.json`, both parse to the same I-JSON value, and the complete expected package is byte-equal |
| `two-support` | Two distinct support derivation IDs remain in the state basis while the state is `support_only` and projection has one `affirm` label |
| `duplicate-node` | Duplicate graph-node identity stops at I0c with `identity.duplicate` |
| `unknown-wrapper-field` | Closed wrapper rejection at S0 with zero counters |
| `wrong-package-language` | Wrong package revision rejection at S0 with zero counters |
| `unknown-contract` | Unknown typed-document contract suppresses verification of a deliberately non-text object and stops at S2 |
| `missing-stance` | A graph node missing required `stance` stops at S2 document-shape verification before identity |
| `missing-reference` | An identity-valid unresolved `DocumentRef` stops at I1 |
| `wrong-modality` | The rule call commits and a contract-valid wrong submitted modality becomes `relation.semantic_mismatch` |
| `unavailable-assumption` | An assumption absent from the caller-pinned basis stops at R1c without dispatch |
| `fuel-exact` | Total semantic fuel equals the basis limit, `4,119`, and succeeds |
| `fuel-crossing` | The same call schedule commits actual fuel `4,119` against limit `4,118` and stops in project |

Each ordinary input is paired with one complete expected outcome or error
package. The two S0 inputs deliberately fail the package-wrapper schema; their
expected error packages remain ordinary schema-valid candidate packages.

The [forbidden mutations](forbidden/README.md) are schema-valid packages that
must differ from the exact specification oracle. They are not assessment
inputs and are not errors emitted by a self-certifying implementation.
