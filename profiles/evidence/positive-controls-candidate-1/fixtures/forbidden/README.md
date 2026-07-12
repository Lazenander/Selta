# Forbidden positive-controls mutations

These packages preserve candidate wrapper and typed-value Selta shapes but
violate the exact dependency functions in document 23. They are independent
conformance falsifiers, not `assess` inputs and not permitted implementation
outputs. Closure-array mutations recompute the outcome identity and root. The
independent-scope mutation removes its retained Text document and therefore
deliberately breaks output document closure while leaving every remaining
typed value shape-valid.

| Mutation | Boundary | Omitted dependency | Occurrences removed |
| --- | --- | --- | --- |
| [checked-witness-omit-theory-assumption](checked-witness-omit-theory-assumption.outcome-package.json) | dependency closure | checked soundness assumption from both target closures | 2 |
| [checked-witness-omit-observation-atom](checked-witness-omit-observation-atom.outcome-package.json) | dependency closure | observation atom EvidenceFact from both target closures | 2 |
| [checked-witness-omit-qualified-attempt](checked-witness-omit-qualified-attempt.outcome-package.json) | dependency closure | qualified observed/include AttemptFact from both target closures | 2 |
| [sequential-omit-population-assumption](sequential-omit-population-assumption.outcome-package.json) | dependency closure | population assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-frame-assumption](sequential-omit-frame-assumption.outcome-package.json) | dependency closure | frame assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-dependence-assumption](sequential-omit-dependence-assumption.outcome-package.json) | dependency closure | dependence assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-calibration-assumption](sequential-omit-calibration-assumption.outcome-package.json) | dependency closure | calibration assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-history-assumption](sequential-omit-history-assumption.outcome-package.json) | dependency closure | history assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-statistic-assumption](sequential-omit-statistic-assumption.outcome-package.json) | dependency closure | statistic assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-stopping-assumption](sequential-omit-stopping-assumption.outcome-package.json) | dependency closure | stopping assumption from the recoverability state and conclusion closures | 2 |
| [sequential-omit-independent-scope](sequential-omit-independent-scope.outcome-package.json) | output document closure | independent Text scope document from the retained output package | 1 |
