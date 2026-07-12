# Mechanics conformance fixtures

These are the complete closed mechanics-profile vectors required by documents
21 and 22. They use environment
`sha256:3cc649a47f2ff7f57d1f55ea8a22623bbe8affc9bd551eb1106212ce84473d18`, the document-19 implementation oracle
`sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0`, and the document-22 implementation oracle
`sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff`.

Every ordinary assessment input uses its complete package `root` as the
caller-supplied `ExpectedBasisRef`. Positive outputs contain the exact state,
dependency closures, conclusions, execution set, resource counters, identities,
and closed output package. Negative outputs contain the exact first error and
charged-work commits. [calls/README.md](calls/README.md) exposes every dispatched
semantic input and output in exact ordinal order. The conformance harness binds
these sources through the separate
[mechanics case records](../../../../conformance/evidence/candidate-1/cases/mechanics/README.md).

The corpus treats an "excluded refutation" as an observed response whose fixture
text is refuting but whose disposition is `exclude`. It stays inspectable in
the trace and receives no core stance: only an atom or explicit rule can create
support or refutation. Closure premise sets include every visited
`EvidenceFact` plus the qualified or operational `FactRef` required by
document 22.

## Ordinary cases

The final column is
`documents/attempts/nodes/edges/semantic-calls/semantic-fuel`.

| Case | Kind | Required observation | Expected root | Exact resources |
|---|---|---|---|---|
| `zero-attempt` | positive | one stop decision over the empty plan set; TraceFact keeps the acquisition reachable | `sha256:0a0b62b306d27f6a0ec823d7c96e6fb6e16017558a48151ed92f17809cc6c5e5` | 23/0/1/1/5/8226 |
| `observed-include-atom` | positive | observed + include is extracted into one support atom under an exact source grant | `sha256:a0340f864ec3a9a3c12fa9a78e5179b27cecac253462240c2429f0053e8c3a24` | 30/1/1/0/7/12050 |
| `unavailable-operational` | positive | unavailable + exclude supports only a separate service-health claim; semantic target remains neither | `sha256:a822c604b6aa52d245217bd21187421145407f169d7d9c19eceeebdd9bd4d8b1` | 34/1/1/1/8/15892 |
| `two-attempt-predecessor` | positive | second plan names the first; excluded refuting response remains in trace and does not become a node | `sha256:aa49c2d87d7be68c9b9b53e172d960060772027925c35af5e254029b39b2ce52` | 35/2/1/0/9/17042 |
| `observed-exclude-opaque-a` | positive | legal observed + exclude and first member of the opaque-content metamorphic pair | `sha256:e89542ad8837616fc4dcef2df7e44e65e4494fb55e92c4c04cc41e1a81de082c` | 31/1/1/1/7/13383 |
| `observed-exclude-opaque-b` | positive | only nested response content and identities induced by it differ from opaque-a | `sha256:17b1043d1ce55a62a2702809e811949e7343c4ed6ad2af51157988dea530720e` | 31/1/1/1/7/13383 |
| `observed-censor` | positive | legal observed + censor combination | `sha256:fad5f7ee5cef2d5a4192bd6dada5282a18b295430424e2e927da555f4281e33f` | 31/1/1/1/7/13380 |
| `malformed-exclude` | positive | legal malformed + exclude combination | `sha256:d276cb8d1e588363200fdde5f036eda04f1d9a1dd508a88397a1ca2dcb29300f` | 32/1/1/1/7/13727 |
| `malformed-censor` | positive | legal malformed + censor combination | `sha256:ca4a14c7d28fc16342727402ea13f7a664dbf9e015f8924c710849c97489beb1` | 32/1/1/1/7/13724 |
| `scoped-accepted` | positive | accepted scoped accounting with exact recorder and attestation authority and complete atom closure | `sha256:13795bb63a30a211af53e3a5962c7ad38cf59f438fdc6df3c8ea588375933125` | 35/1/1/0/8/13130 |
| `forwarding-recursive` | positive | forwarding EvidenceFact recursively reaches atom, source, all policies, recorder, and attestation | `sha256:b7d5dd089d4f0a0d64eb6b2e1b1360d13d2de4b6382e54d25cf4af88d64fa7b7` | 36/1/2/1/9/16156 |
| `missing-source-grant` | negative | the exact observed/include atom from the positive case lacks only its source grant | `sha256:9fdb0c87cc7419c9e5ce634e1d436b0271be239cbc874fa7d8602e009159348d` | 29/1/1/0/0/0 |
| `invalid-atom-unavailable` | negative | an unavailable attempt cannot directly feed an atom; cross-record qualification fails in R1e | `sha256:3bcbb8f4a1bb96fb1a69a56781c2dfed8febd1c4f1bdda927887b589e86c0a85` | 29/1/1/0/0/0 |
| `malformed-include` | negative | forbidden malformed + include combination stops in R1d | `sha256:b4b67e46dbc3518a61dce316f3cc328d0addd01d8b5319c5b1a9b59864d268af` | 31/1/1/1/0/0 |
| `unavailable-include` | negative | forbidden unavailable + include combination stops in R1d | `sha256:0eac0f13246c6bbd2d3df6004b68e2e9e7268b931508645d9033534ad533c3a5` | 30/1/1/1/0/0 |
| `unavailable-censor` | negative | forbidden unavailable + censor combination stops in R1d | `sha256:adab3db256dca9fc47ebcf1255ccd922cfcee196a6bc6d74f6d82cb87411e6d9` | 30/1/1/1/0/0 |
| `missing-recorder` | negative | accepted scoped record lacks its exact recorder grant | `sha256:3d7c6ad6c61108d15b1a7bbc151fd5c6415a1a4d68da6d95e9ec9f3edd18357a` | 34/1/1/0/0/0 |
| `missing-attestation-grant` | negative | scoped verifier binding lacks its exact semantic grant | `sha256:26611d2b567040c3a1d857e695ed8ae769f8e49f0dc50e61560ff7ac1f73aebc` | 34/1/1/0/0/0 |
| `scoped-rejected` | negative | contract-valid rejected attestation dispatches and mismatches the required accepted result | `sha256:b0933d92e437c06dce9d0c1468725b98274bd753baa385dffeabb5b79424cd33` | 34/1/1/0/5/8587 |
| `wrong-manifest-view` | negative | stop policy snapshot carries a schema-valid but wrong manifest view and fails R1d | `sha256:3a2019bc986fd9f1f3b81ad8f39ea36ad1f183e4ce6c31d064f3e341638bbcee` | 30/1/1/1/0/0 |
| `wrong-observed-plans` | negative | stop decision locally matches its input but omits the complete plan set and fails R1e | `sha256:f3a13822aaa32197bae87693540621dcf09e1514168c27f40c6f8ec077faec86` | 30/1/1/1/0/0 |
| `wrong-selection-plan` | negative | selection snapshot names a schema-valid wrong plan and fails cross-record R1e | `sha256:71f7e91cb5ff632b1131f62efde849ee836f339629ccd7127dd1c40d94af4161` | 30/1/1/1/0/0 |
| `wrong-selection-outcome` | negative | selection snapshot carries a schema-valid wrong outcome and fails cross-record R1e | `sha256:1bb099ea9c588934045bcb482ed94f5b5cfd73b1269b11bf412878844823407f` | 31/1/1/1/0/0 |
| `noncanonical-predecessors` | negative | one stored predecessor set is descending; I0a suppresses later identity and relation defects | `sha256:06397dd34a34db93f591aaa0fe0d87c7afd0544c84142ef99c1cbf9ec6455855` | 38/0/0/0/0/0 |
| `invalid-stop-transition` | negative | {stop:false} is identity-closed but fails the StopTransition contract at S2 | `sha256:6f9436ac3aee24693121891298f48c6ce831ec1f17944517a7bc1344fadd6b38` | 30/0/0/0/0/0 |

## Conformance-control cases

Controls are conformance-only and never enter candidate package bytes,
identities, resources, or product interfaces. Each record passes the S2 case or
oracle Selta schema.

| Case | Required observation | Case record | Oracle record |
|---|---|---|---|
| `mechanics-operational-forbidden-read` | forbidden nested-document dereference traps before value exposure | [case](../../../../conformance/evidence/candidate-1/cases/mechanics/mechanics-operational-forbidden-read.json) | [oracle](../../../../conformance/evidence/candidate-1/oracles/mechanics/mechanics-operational-forbidden-read.json) |
| `mechanics-opaque-payload-mutant-a` | payload-dependent semantic output is rejected after dispatch | [case](../../../../conformance/evidence/candidate-1/cases/mechanics/mechanics-opaque-payload-mutant-a.json) | [oracle](../../../../conformance/evidence/candidate-1/oracles/mechanics/mechanics-opaque-payload-mutant-a.json) |
| `mechanics-opaque-payload-mutant-b` | payload-dependent semantic output is rejected after dispatch | [case](../../../../conformance/evidence/candidate-1/cases/mechanics/mechanics-opaque-payload-mutant-b.json) | [oracle](../../../../conformance/evidence/candidate-1/oracles/mechanics/mechanics-opaque-payload-mutant-b.json) |

## Coverage map

- `zero-attempt` proves the empty stop transition while a `TraceFact` keeps
  the acquisition in the exact graph inventory.
- `observed-include-atom`, `missing-source-grant`, and
  `invalid-atom-unavailable` isolate extractor and source authority.
- `unavailable-operational` preserves a semantic `neither` target while an
  explicit operational rule supports a distinct service-health target.
- `two-attempt-predecessor` binds predecessor closure and preserves an
  excluded response without coercing it into evidence.
- the six legal and three illegal outcome/disposition combinations are indexed
  in [positive/README.md](positive/README.md) and
  [negative/README.md](negative/README.md).
- `scoped-accepted`, the two missing-authority cases, and
  `scoped-rejected` cover recorder and attestation admission and dispatch.
- `forwarding-recursive` contains complete recursive policy, source,
  recorder, attestation, atom, and forwarding closure branches.
- the opaque pair, forbidden-read trap, and payload-dependent mutants establish
  shallow operational resolution and its falsifier boundary.
- six policy-boundary inputs cover wrong manifest view, observed plans,
  selection plan, selection outcome, predecessor order, and stop shape.

No vector asserts that these echo/control policies are empirically sound.
