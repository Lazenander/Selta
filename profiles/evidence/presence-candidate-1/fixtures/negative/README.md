# Negative assessment vectors

Each case is a complete `assess` input paired with the exact one-document error
package required by documents 16 and 20. The caller supplies the input
package's `root` as `ExpectedBasisRef`; a case is not testing caller mismatch.

| Case | Expected error | Boundary exercised |
|---|---|---|
| `duplicate-document` | `identity.duplicate` | Duplicate canonical document-set member |
| `wrong-target` | `relation.invalid_graph` | Graph claim inventory differs from basis targets |
| `unauthorized-projection` | `relation.unauthorized` | Valid projection binding lacks an exact semantic grant |
| `wrong-role-projection` | `relation.invalid_binding` | Affirm/deny label documents occupy the wrong parameter roles |
| `equal-label-projection` | `relation.invalid_binding` | The two projection roles name one label document |
| `wrong-assumption` | `relation.invalid_derivation` | A schema-valid, available assumption differs from the rule parameter |

All input and expected packages pass their stable Selta shape contracts. The
rejection is therefore relational or identity behavior of the candidate
assessment language, not a disguised schema failure. Expected error records,
resource counters, execution sets, outcome digests, and package roots are
normative.
