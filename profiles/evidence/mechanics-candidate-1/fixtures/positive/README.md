# Positive mechanics assessments

Every input is identity-valid, canonically ordered, document-closed, and paired
with its complete concluded outcome package.

## Outcome/disposition matrix

| Outcome | Disposition | Fixture | Evidence path |
|---|---|---|---|
| observed | include | [observed-include-atom](observed-include-atom.input-package.json) | authorized extractor atom |
| observed | exclude | [opaque-a](observed-exclude-opaque-a.input-package.json) | operational AttemptFact only |
| observed | censor | [observed-censor](observed-censor.input-package.json) | operational AttemptFact only |
| malformed | exclude | [malformed-exclude](malformed-exclude.input-package.json) | operational AttemptFact only |
| malformed | censor | [malformed-censor](malformed-censor.input-package.json) | operational AttemptFact only |
| unavailable | exclude | [unavailable-operational](unavailable-operational.input-package.json) | operational AttemptFact for service health |

The second opaque member changes only the nested observed response and the
identities induced by it. Its operational semantic output, state name, and
labels equal opaque-a; its completed attempt, acquisition, graph, basis, node,
and visible provenance basis remain distinct.
