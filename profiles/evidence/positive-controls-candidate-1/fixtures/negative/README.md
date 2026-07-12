# Negative positive-controls assessments

Every input passes stable Selta package and typed-value shape checks. R1 cases
dispatch no relation. The checked refutation, rejected attestation, and
substantive-output cases reach R2 and charge the exact calls shown below.

| Fixture | Code | Exact path | Boundary | Calls/fuel |
| --- | --- | --- | --- | --- |
| `checked-witness-invalid-submitted` | `relation.invalid_derivation` | `/input/documents/13/value/nodes/0` | a submitted witness whose UTF-8 SHA-256 misses the proposition fails R1d without dispatch | 0/0 |
| `checked-witness-valid-refute` | `relation.semantic_mismatch` | `/input/documents/7/value/nodes/1` | the support-only checker result cannot match a submitted refuting derivation | 7/13995 |
| `checked-witness-missing-checker-grant` | `relation.unauthorized` | `/input/documents/27/value/nodes/0/rule` | the exact checker semantic grant is absent at R1c | 0/0 |
| `checked-witness-unavailable-theory-assumption` | `relation.invalid_assumption` | `/input/documents/2/value/nodes/1/assumptions/0` | the checker derivation names a theory assumption absent from the caller basis | 0/0 |
| `checked-witness-wrong-theory-statement` | `relation.invalid_derivation` | `/input/documents/14/value/nodes/1` | the available soundness assumption statement differs from the rule parameter document | 0/0 |
| `checked-witness-wrong-theory-scope` | `relation.invalid_derivation` | `/input/documents/3/value/nodes/1` | the available soundness assumption is scoped to the observation claim instead of the existential claim | 0/0 |
| `sequential-wrong-scope-contract` | `relation.invalid_binding`<br>`relation.invalid_binding` | `/input/documents/31/value/trace/accounting/verifier`<br>`/input/documents/6/value/nodes/0/rule` | the basis scope resolves as Unit rather than non-empty Text, invalidating both dependent positive-control bindings in R1b | 0/0 |
| `sequential-expectation-scope-mismatch` | `relation.invalid_binding` | `/input/documents/6/value/trace/accounting/verifier` | the expectation and record agree with each other but not with the asserted basis scope, so R1b owns the failure | 0/0 |
| `sequential-checkpoint-mismatch` | `relation.invalid_accounting` | `/input/documents/27/value/trace/accounting` | a checkpoint-only record/expectation mismatch reaches R1d invalid_accounting | 0/0 |
| `sequential-missing-recorder-grant` | `relation.unauthorized` | `/input/documents/21/value/trace/accounting` | the exact recorder/scope grant is absent at R1c | 0/0 |
| `sequential-missing-attestation-grant` | `relation.unauthorized` | `/input/documents/35/value/trace/accounting/verifier` | the exact sequential-attestation semantic grant is absent at R1c | 0/0 |
| `sequential-rejected-result` | `relation.semantic_mismatch` | `/input/documents/10/value/trace/accounting/attestation` | the contract-valid rejected attestation dispatches but cannot satisfy the scoped accepted-result requirement | 4/6449 |
| `sequential-unavailable-assumption` | `relation.invalid_assumption` | `/input/documents/28/value/nodes/0/assumptions/1` | the population role ID is named by the basis and derivation but absent from caller assumptions | 0/0 |
| `sequential-omitted-assumption` | `relation.invalid_derivation` | `/input/documents/36/value/nodes/0` | the derivation omits the available population role and fails the exact seven-ID input domain | 0/0 |
| `sequential-duplicate-role-id` | `relation.invalid_binding` | `/input/documents/7/value/nodes/0/rule` | population and frame reuse one assumption ID, violating pairwise distinctness in R1b | 0/0 |
| `sequential-role-mismatch` | `relation.invalid_derivation` | `/input/documents/31/value/nodes/0` | the population ID resolves to a structurally valid frame-role statement | 0/0 |
| `sequential-statement-contract-mismatch` | `relation.invalid_derivation` | `/input/documents/25/value/nodes/0` | the population assumption statement resolves under Text rather than SequentialAssumption | 0/0 |
| `sequential-assumption-scope-mismatch` | `relation.invalid_derivation` | `/input/documents/21/value/nodes/0` | the population assumption scope differs from arguments.value.scope | 0/0 |
| `sequential-substantive-target-binding` | `relation.invalid_binding` | `/input/documents/5/value/nodes/0/rule` | the recovery rule parameter is rebound to the substantive claim and fails R1b | 0/0 |
| `sequential-substantive-target-output` | `relation.semantic_mismatch` | `/input/documents/18/value/nodes/0` | valid recoverability parameters cannot match a submitted evidence result for the substantive target | 5/10860 |
| `sequential-missing-recovery-grant` | `relation.unauthorized` | `/input/documents/6/value/nodes/0/rule` | the exact sequential-recovery semantic grant is absent at R1c | 0/0 |

The wrong-scope-contract case deliberately emits two peer R1b records: the
same Unit-typed basis scope invalidates the sequential attestation binding and
the recovery-rule binding. This follows the document-20 peer-error schedule;
neither relation dispatches.
