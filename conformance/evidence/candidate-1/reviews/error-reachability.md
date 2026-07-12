# S2 error reachability and coverage review

> **Status: fixture inventory instantiated; blind model execution pending.**
> This is a durable conformance review, not candidate semantics, a proof of
> calibration, or a claim that either independent model passes. Documents
> [15](../../../../docs/15-evidence-language.md),
> [16](../../../../docs/16-evidence-semantics.md),
> [20](../../../../docs/20-evidence-error-catalog.md), and
> [21](../../../../docs/21-s2-conformance-plan.md) remain authoritative.

## Classification

Document 20 has 38 distinct codes and 48 distinct `(phase, code)` rows. The
candidate-1 corpus classifies them as:

- 35 **wire** rows, reached by raw bytes or a schema-valid package under a
  conforming selected environment;
- 13 **control** rows, reached only through the named private document-21
  control without changing ordinary candidate semantics; and
- 0 **unreachable** rows.

`wire` does not mean that an input is well formed at every later phase. It
means the ordinary boundary can reach the listed first error. `control` does
not make the control part of the evidence DSL or product API. No fixture name
alone is completion evidence: the manifest, two blind prediction ledgers, and
arbiter comparison remain required.

## Exact phase/code matrix

| # | Phase | Code | Reachability | Control | Corpus case IDs |
|---:|---|---|---|---|---|
| 01 | `parse` | `package.syntax` | wire | — | `boundary-raw-syntax`, `boundary-raw-invalid-utf8`, `boundary-raw-lone-surrogate`, `boundary-raw-noncharacter`, `boundary-raw-number-out-of-range`, `boundary-raw-trailing-data` |
| 02 | `parse` | `package.duplicate_name` | wire | — | `precedence-parse-earlier-duplicate-before-later-syntax` |
| 03 | `parse` | `resource.outer_exceeded` | wire | — | `precedence-parse-outer-before-duplicate`, `boundary-depth-exceeded`, `boundary-values-exceeded`, `boundary-decoded-string-exceeded` |
| 04 | `schema` | `package.shape` | wire | — | `presence-unknown-wrapper-field`, `presence-wrong-package-language` |
| 05 | `schema` | `package.environment_mismatch` | wire | — | `boundary-environment-mismatch` |
| 06 | `schema` | `package.unknown_contract` | wire | — | `presence-unknown-contract` |
| 07 | `schema` | `package.document_shape` | wire | — | `presence-missing-stance`, `mechanics-invalid-stop-transition` |
| 08 | `schema` | `package.verification_error` | control | `verification_fault` | `control-schema-s0-verification-fault`, `control-schema-s2-verification-fault` |
| 09 | `identity` | `identity.noncanonical` | wire | — | `boundary-unsafe-int`, `precedence-identity-noncanonical-before-mismatch`, `boundary-pointer-normalization-dedup`, `mechanics-noncanonical-predecessors` |
| 10 | `identity` | `identity.mismatch` | wire | — | `identity-wrong-domain`, `identity-wrong-revision`, `precedence-identity-mismatch-before-duplicate` |
| 11 | `identity` | `identity.collision` | control | `identity_result` | `control-input-identity-collision` |
| 12 | `identity` | `identity.duplicate` | wire | — | `presence-duplicate-document`, `presence-duplicate-node`, `precedence-identity-duplicate-before-unknown` |
| 13 | `identity` | `identity.unknown_semantics` | wire | — | `precedence-identity-unknown-before-reference` |
| 14 | `identity` | `package.missing_reference` | wire | — | `presence-missing-reference` |
| 15 | `identity` | `package.contract_mismatch` | wire | — | `identity-reference-contract-mismatch` |
| 16 | `identity` | `package.reference_cycle` | control | `identity_result` | `control-document-reference-cycle` |
| 17 | `identity` | `package.unreachable_document` | wire | — | `identity-unreachable-document` |
| 18 | `relation` | `basis.mismatch` | wire | — | `relation-basis-mismatch` |
| 19 | `relation` | `relation.unauthorized` | wire | — | `presence-unauthorized-projection`, `mechanics-missing-source-grant`, `mechanics-missing-recorder`, `mechanics-missing-attestation-grant` |
| 20 | `relation` | `relation.invalid_binding` | wire | — | `presence-equal-label-projection`, `presence-wrong-role-projection`, `precedence-relation-binding-before-authority` |
| 21 | `relation` | `relation.kind_mismatch` | wire | — | `relation-kind-mismatch` |
| 22 | `relation` | `relation.semantic_mismatch` | wire | — | `presence-wrong-modality`, `mechanics-scoped-rejected` |
| 23 | `relation` | `relation.claim_noncanonical` | control | `semantic_output` | `control-claim-noncanonical` |
| 24 | `relation` | `relation.invalid_assumption` | wire | — | `presence-unavailable-assumption` |
| 25 | `relation` | `relation.invalid_decision` | wire | — | `mechanics-wrong-manifest-view` |
| 26 | `relation` | `relation.invalid_attempt` | wire | — | `mechanics-malformed-include`, `mechanics-unavailable-include`, `mechanics-unavailable-censor` |
| 27 | `relation` | `relation.invalid_trace` | wire | — | `mechanics-wrong-observed-plans`, `mechanics-wrong-selection-plan`, `mechanics-wrong-selection-outcome` |
| 28 | `relation` | `relation.invalid_accounting` | wire | — | `positive-controls-sequential-checkpoint-mismatch` |
| 29 | `relation` | `relation.invalid_graph` | wire | — | `presence-wrong-target` |
| 30 | `relation` | `relation.invalid_atom` | wire | — | `mechanics-invalid-atom-unavailable` |
| 31 | `relation` | `relation.invalid_derivation` | wire | — | `presence-wrong-assumption` |
| 32 | `relation` | `relation.cycle` | control | `identity_result` | `control-graph-premise-cycle` |
| 33 | `relation` | `resource.basis_exceeded` | wire | — | `resource-relation-static-exceeded`, `resource-relation-call-limit` |
| 34 | `relation` | `evaluation.invalid_output` | control | `semantic_output` | `control-relation-invalid-output` |
| 35 | `relation` | `evaluation.fuel_exhausted` | wire | — | `resource-relation-fuel-crossing` |
| 36 | `relation` | `evaluation.arithmetic_overflow` | control | `semantic_output_bytes` | `mechanics-arithmetic-overflow-relation` |
| 37 | `interpret` | `identity.collision` | control | `identity_result` | `control-state-identity-collision` |
| 38 | `interpret` | `resource.basis_exceeded` | wire | — | `resource-interpret-state-limits-exceeded`, `resource-interpret-call-limit` |
| 39 | `interpret` | `evaluation.invalid_output` | control | `semantic_output` | `control-interpret-invalid-output` |
| 40 | `interpret` | `evaluation.fuel_exhausted` | wire | — | `resource-relation-fuel-exact-interpret-crossing` |
| 41 | `interpret` | `evaluation.arithmetic_overflow` | control | `semantic_output_bytes` | `mechanics-arithmetic-overflow-interpret` |
| 42 | `project` | `resource.basis_exceeded` | wire | — | `resource-project-labels-exceeded`, `resource-project-call-limit` |
| 43 | `project` | `evaluation.invalid_output` | control | `semantic_output` | `control-project-invalid-output` |
| 44 | `project` | `evaluation.fuel_exhausted` | wire | — | `presence-fuel-crossing`, `resource-interpret-fuel-exact-project-crossing` |
| 45 | `project` | `evaluation.arithmetic_overflow` | control | `semantic_output_bytes` | `mechanics-arithmetic-overflow-project` |
| 46 | `output` | `identity.collision` | control | `identity_result` | `control-outcome-identity-collision` |
| 47 | `output` | `output.too_large` | wire | — | `output-success-too-large`, `output-o0-too-large-skips-o1` |
| 48 | `output` | `error.truncated` | wire | — | `boundary-error-truncation` |

The 13 control rows are exactly rows 08, 11, 16, 23, 32, 34, 36, 37, 39,
41, 43, 45, and 46. A future classification change must preserve the total of
48 rows and supply an ordinary fixture or a durable unreachable proof; prose
alone cannot change the manifest.

## Ordering and commit boundaries

The matrix specifies intended reachability. The following cases are designed
to make the important cross-row ordering and resource-commit claims observable
in both blind runs:

| Boundary | Cases | Specified observation |
|---|---|---|
| P0 before P1 | `precedence-parse-outer-before-duplicate` | Raw bytes stop before decoding or duplicate inspection. |
| P1 stop order | `precedence-parse-earlier-duplicate-before-later-syntax` | The earlier duplicate stops before a later syntax defect. |
| Outer exact/crossing | `boundary-raw-bytes-exact`, `boundary-depth-exact`, `boundary-depth-exceeded`, `boundary-values-exact`, `boundary-values-exceeded`, `boundary-decoded-string-exact`, `boundary-decoded-string-exceeded` | Exact ceilings proceed; the first crossing stops with no later work. |
| S0–S2 | `control-schema-s0-verification-fault`, `boundary-environment-mismatch`, `presence-unknown-contract`, `control-schema-s2-verification-fault`, `presence-missing-stance` | Wrapper, environment, unknown-contract suppression, verifier fault, and document-shape ownership remain distinct. |
| I0 ordering | `boundary-unsafe-int`, `precedence-identity-noncanonical-before-mismatch`, `identity-wrong-domain`, `identity-wrong-revision`, `control-input-identity-collision`, `precedence-identity-mismatch-before-duplicate`, `precedence-identity-duplicate-before-unknown` | Separate fixtures reach canonicality, domain/revision recomputation, and collision; dual-fault fixtures witness I0a before I0b, I0b before I0c, and I0c before I0d. |
| I1–I3 | `precedence-identity-unknown-before-reference`, `identity-reference-contract-mismatch`, `precedence-reference-before-cycle`, `control-document-reference-cycle`, `identity-unreachable-document` | Reference validity, cycle, and reachability do not mask one another. |
| R0–R1f | `relation-basis-mismatch`, `relation-kind-mismatch`, `precedence-relation-binding-before-authority`, `precedence-relation-authority-before-local`, `precedence-relation-local-before-cross-record`, `precedence-relation-cross-record-before-cycle`, `control-graph-premise-cycle` | Each earlier relation substage suppresses every later check and dispatch. |
| Static basis | `resource-relation-static-exact`, `resource-relation-static-exceeded` | R1f exact equality is accepted; crossing fails before dispatch. |
| Call limits | `resource-relation-call-limit`, `resource-interpret-call-limit`, `resource-project-call-limit` | A denied call neither dispatches nor increments calls or fuel. |
| Fuel commit | `resource-relation-fuel-crossing`, `resource-relation-fuel-exact-interpret-crossing`, `resource-interpret-fuel-exact-project-crossing`, `presence-fuel-exact`, `presence-fuel-crossing` | Representable crossing fuel commits before the phase-local limit error. |
| Fuel before output checks | `precedence-relation-fuel-before-invalid-output`, `precedence-interpret-fuel-before-invalid-output`, `precedence-project-fuel-before-invalid-output` | Step-3 accounting precedes output verification and semantic comparison. |
| State generation | `resource-state-equal-digest-keeps-charge`, `control-state-identity-collision`, `resource-interpret-state-limits-exact`, `resource-interpret-state-limits-exceeded` | State charge, retained-map reuse/collision, and T2 limits occur in their fixed order. |
| Projection | `resource-project-labels-exact`, `resource-project-labels-exceeded` | Label count commits before J1 applies its basis limit. |
| O0/O1 | `output-success-too-large`, `output-o0-too-large-skips-o1`, `output-error-skips-o1`, `control-outcome-identity-collision` | O0 replacement and error outcomes skip concluded insertion; accepted conclusions alone reach O1. |
| Pointer/truncation | `boundary-pointer-normalization-dedup`, `boundary-error-truncation` | Normalization precedes tuple deduplication; sorted unique errors precede the 255-plus-marker truncation. |

`resource-relation-static-exact` is designed to witness equality acceptance at
R1f, then deliberately reaches a T2 generated-state limit. It is not advertised
as a concluded assessment. Similarly, exact fuel cases target the named
boundary; they need not erase a later, independently owned crossing.

## Controls and honest limits

The strengthened `presence-unknown-contract` carries a deliberately non-text
value. It has no verification-fault control: S2 must stop at the unknown
contract, so both value verification and its latent identity mismatch remain
unobserved.

`evaluation.arithmetic_overflow` is not unconditionally unreachable. A
nonconforming host or `semantic_output_bytes` control can supply a measured
output length whose checked addition overflows before the fuel comparison.
The three arithmetic cases use `SafeInt::MAX`, count the dispatched call, keep
the prior representable fuel subtotal, and set a fuel limit at least that
subtotal but below the hypothetical mathematical total. This is designed to
distinguish overflow-before-fuel from an accidentally convenient limit.

The parser corpus observes an earlier duplicate before a later syntax fault.
A distinct same-byte-offset duplicate-versus-syntax input is not constructible
under the fixed tokenization; that tie order remains a formal specification
rule and is not misreported as empirical coverage.

`forbidden_boundary: "concluded_outcome_insertion"` is an exact-use negative
assertion, not a candidate error or manifest reachability kind. Zero visits is
success. The fixed O0 replacement is measured without recursively entering O1.

The corpus now supplies a record designed for every row and every listed
ordering edge. Promotion still requires manifest closure, blind execution by
both independent models, exact oracle comparison, review sealing, and
stable-workspace evidence.
