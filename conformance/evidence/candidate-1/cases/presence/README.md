# Presence conformance case records

These records bind existing profile sources by ordinary raw-byte SHA-256; they do not copy package bytes. Oracles are withheld from implementer kits.

| Case | Input source | Oracle source or assertion |
|---|---|---|
| `presence-neither` | `profiles/evidence/presence-candidate-1/fixtures/neither.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/neither.expected-outcome-package.json` |
| `presence-support-only` | `profiles/evidence/presence-candidate-1/fixtures/support_only.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/support_only.expected-outcome-package.json` |
| `presence-refute-only` | `profiles/evidence/presence-candidate-1/fixtures/refute_only.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/refute_only.expected-outcome-package.json` |
| `presence-both` | `profiles/evidence/presence-candidate-1/fixtures/both.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/both.expected-outcome-package.json` |
| `presence-duplicate-document` | `profiles/evidence/presence-candidate-1/fixtures/negative/duplicate-document.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/duplicate-document.expected-error-package.json` |
| `presence-equal-label-projection` | `profiles/evidence/presence-candidate-1/fixtures/negative/equal-label-projection.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/equal-label-projection.expected-error-package.json` |
| `presence-unauthorized-projection` | `profiles/evidence/presence-candidate-1/fixtures/negative/unauthorized-projection.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/unauthorized-projection.expected-error-package.json` |
| `presence-wrong-assumption` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-assumption.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-assumption.expected-error-package.json` |
| `presence-wrong-role-projection` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-role-projection.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-role-projection.expected-error-package.json` |
| `presence-wrong-target` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-target.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/negative/wrong-target.expected-error-package.json` |
| `presence-duplicate-node` | `profiles/evidence/presence-candidate-1/fixtures/s2/duplicate-node.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/duplicate-node.expected-error-package.json` |
| `presence-fuel-crossing` | `profiles/evidence/presence-candidate-1/fixtures/s2/fuel-crossing.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/fuel-crossing.expected-error-package.json` |
| `presence-fuel-exact` | `profiles/evidence/presence-candidate-1/fixtures/s2/fuel-exact.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/fuel-exact.expected-outcome-package.json` |
| `presence-missing-reference` | `profiles/evidence/presence-candidate-1/fixtures/s2/missing-reference.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/missing-reference.expected-error-package.json` |
| `presence-replay-identical` | `profiles/evidence/presence-candidate-1/fixtures/neither.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/neither.expected-outcome-package.json` |
| `presence-replay-equivalent` | `profiles/evidence/presence-candidate-1/fixtures/s2/replay-equivalent.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/replay-equivalent.expected-outcome-package.json` |
| `presence-two-support` | `profiles/evidence/presence-candidate-1/fixtures/s2/two-support.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/two-support.expected-outcome-package.json` |
| `presence-unavailable-assumption` | `profiles/evidence/presence-candidate-1/fixtures/s2/unavailable-assumption.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/unavailable-assumption.expected-error-package.json` |
| `presence-unknown-contract` | `profiles/evidence/presence-candidate-1/fixtures/s2/unknown-contract.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/unknown-contract.expected-error-package.json` |
| `presence-unknown-wrapper-field` | `profiles/evidence/presence-candidate-1/fixtures/s2/unknown-wrapper-field.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/unknown-wrapper-field.expected-error-package.json` |
| `presence-wrong-modality` | `profiles/evidence/presence-candidate-1/fixtures/s2/wrong-modality.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/wrong-modality.expected-error-package.json` |
| `presence-wrong-package-language` | `profiles/evidence/presence-candidate-1/fixtures/s2/wrong-package-language.input-package.json` | `profiles/evidence/presence-candidate-1/fixtures/s2/wrong-package-language.expected-error-package.json` |
| `presence-replay-equality` | comparison | exact per-model package JCS equality across an identical replay and a canonically equivalent raw spelling |
| `presence-conflict-vs-absence` | comparison | distinct state, equal cautious labels |
| `presence-support-multiplicity` | comparison | distinct provenance basis, equal state and labels |
