# Candidate-1 finite named corpus

This directory contains the exact finite calculations required by document 21.
Each `*.input.json` value is admitted by `finite-world.schema.json`; its paired
`*.expected.json` value is admitted by `finite-result.schema.json` and is the
unique result of the relational evaluator defined in the parent README.

The corpus covers only the algebraic countermodels from document 14:

| Named case | Input kind | Exact observation |
|---|---|---|
| `CASCADE-FALSE-REJECT` | `repeat_event` | false rejection at depths 1, 2, and 4 |
| `CORRELATED-MAJORITY` | `world_table` | shared-channel majority accuracy `3/5` |
| `CORRELATED-UNANIMITY` | `world_table` | accuracy conditional on agreement `4/5` |
| `INCOMPETENT-INDEPENDENT` | `world_table` | three-voter majority accuracy `44/125` |
| `RECURSIVE-FIXED-POINT` | `transition` | one wrong state remains fixed |
| `OPTIONAL-STOP` | `repeat_event` | one attempt `1/2`, up to two `3/4` |
| `BEST-OF-N-PROXY` | `world_table` | the higher proxy selects the false candidate |
| `GENUINE-DISAGREEMENT` | `world_table` | two labels retain two perspective lineages |
| `VALID-LOG-OMISSION` | `world_table` | one exposed log is compatible with two worlds |
| `ADAPTIVE-HOLDOUT` | `repeat_event` | exact compact `1 - (1023/1024)^1024` |

Probability events are derived from row facts by canonical DNF selectors; no
fixture carries a preselected event row list. `INCOMPETENT-INDEPENDENT` lists
all eight Boolean product worlds. Its row weights are the exact integer
products of correct weight `2` and incorrect weight `3`, so the majority event
has weight `12 + 12 + 12 + 8 = 44` out of `(2 + 3)^3 = 125`.

Acquisition, evidence identity, conflict preservation, checked-witness, and
assumption-recoverability cases are assessment cases under their respective
conformance environments and do not belong in this directory.
