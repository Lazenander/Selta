# Forbidden conformance mutations

These files are deliberately wrong implementation outputs. They are not
`assess` inputs and are not error outcomes. A reference implementation is
conformant only when it produces the positive oracle value rather than the
corresponding mutation.

| File | Shape status | Required distinction |
|---|---|---|
| `support_only.wrong-label.projection-output.json` | Passes projection-output schema | Differs from the specified `affirm` result |
| `support_only.wrong-target.projection-output.json` | Passes projection-output schema | Differs from the input target |
| `neither.empty-label.projection-output.json` | Fails projection-output schema | Cannot represent abstention as an empty set |
| `support_only.bad-closure.outcome-package.json` | Package and typed values pass stable schemas | Differs from the exact dependency function |

The schema-valid mutations demonstrate why shape verification is necessary but
not sufficient. The candidate runtime does not claim to certify its own native
implementation by comparing a function with itself; these are independent
conformance-oracle cases.
