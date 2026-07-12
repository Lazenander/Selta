# Negative mechanics assessments

All inputs except `invalid-stop-transition` pass every stable Selta shape
contract. That deliberate S2 boundary fails only the StopTransition document;
its wrapper and all other typed values pass. Identity, relation, and R2 cases
are otherwise schema-admitted and closed.

| Case | Phase | Code | Exact path | Boundary |
|---|---|---|---|---|
| `missing-source-grant` | `relation` | `relation.unauthorized` | `/input/documents/13/value/nodes/0/attempt` | the exact observed/include atom from the positive case lacks only its source grant |
| `invalid-atom-unavailable` | `relation` | `relation.invalid_atom` | `/input/documents/15/value/nodes/0` | an unavailable attempt cannot directly feed an atom; cross-record qualification fails in R1e |
| `malformed-include` | `relation` | `relation.invalid_attempt` | `/input/documents/8/value/trace/attempts/0` | forbidden malformed + include combination stops in R1d |
| `unavailable-include` | `relation` | `relation.invalid_attempt` | `/input/documents/16/value/trace/attempts/0` | forbidden unavailable + include combination stops in R1d |
| `unavailable-censor` | `relation` | `relation.invalid_attempt` | `/input/documents/10/value/trace/attempts/0` | forbidden unavailable + censor combination stops in R1d |
| `missing-recorder` | `relation` | `relation.unauthorized` | `/input/documents/19/value/trace/accounting` | accepted scoped record lacks its exact recorder grant |
| `missing-attestation-grant` | `relation` | `relation.unauthorized` | `/input/documents/19/value/trace/accounting/verifier` | scoped verifier binding lacks its exact semantic grant |
| `scoped-rejected` | `relation` | `relation.semantic_mismatch` | `/input/documents/26/value/trace/accounting/attestation` | contract-valid rejected attestation dispatches and mismatches the required accepted result |
| `wrong-manifest-view` | `relation` | `relation.invalid_decision` | `/input/documents/17/value/trace/decisions/2` | stop policy snapshot carries a schema-valid but wrong manifest view and fails R1d |
| `wrong-observed-plans` | `relation` | `relation.invalid_trace` | `/input/documents/3/value/trace` | stop decision locally matches its input but omits the complete plan set and fails R1e |
| `wrong-selection-plan` | `relation` | `relation.invalid_trace` | `/input/documents/5/value/trace` | selection snapshot names a schema-valid wrong plan and fails cross-record R1e |
| `wrong-selection-outcome` | `relation` | `relation.invalid_trace` | `/input/documents/2/value/trace` | selection snapshot carries a schema-valid wrong outcome and fails cross-record R1e |
| `noncanonical-predecessors` | `identity` | `identity.noncanonical` | `/input/documents/28/value/trace/attempts/2/predecessors` | one stored predecessor set is descending; I0a suppresses later identity and relation defects |
| `invalid-stop-transition` | `schema` | `package.document_shape` | `/input/documents/2/value` | {stop:false} is identity-closed but fails the StopTransition contract at S2 |

The noncanonical predecessor case deliberately leaves the later embedded plan
identity inconsistent: I0a is the exact first substage and suppresses I0b and
all relational diagnostics. Its attempt and graph counters therefore remain
zero while document count and bytes are committed.
