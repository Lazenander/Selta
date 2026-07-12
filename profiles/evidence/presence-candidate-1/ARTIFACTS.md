# Presence candidate artifact resolver

This is the complete digest-to-path inventory needed to admit
`environment.json` without searching the source tree. Paths are repository
locations, not identity preimages. Digests and domain tags are normative;
location is replaceable when the resolved content has the same identity.

## Root and specification artifacts

| Identity | Domain | Repository artifact |
|---|---|---|
| `sha256:0c153635b88d6dee3f1301f1a46b20d1a4439d5f7b755fe276e8d93a8ccdf8a3` | `selta.evidence.environment/candidate-1` | [environment.json](environment.json) |
| `sha256:71806dd912b31da7bbd8c03365654d28d6823715d3b8ca1c17b87b0c382fe0e5` | `selta.evidence.language-specification/candidate-1` | [docs/15-evidence-language.md](../../../docs/15-evidence-language.md) |
| `sha256:87c2564bfa13553c9dddb2d3726203239ad1aebec5f12b3cf96a9a52d32536d0` | `selta.evidence.core-semantics/candidate-1` | [docs/16-evidence-semantics.md](../../../docs/16-evidence-semantics.md) |
| `sha256:923879b5172a4b0f35ce37aeedaece6227716614dba1581e6635943a3e0dd186` | `selta.evidence.error-catalog/candidate-1` | [docs/20-evidence-error-catalog.md](../../../docs/20-evidence-error-catalog.md) |
| `sha256:f08f66de43aceef481f21bb4b556ab00eee85584885e077c01c44fc09aa36b6c` | `selta.evidence.semantic-specification/candidate-1` | [docs/19-presence-reference-profile.md](../../../docs/19-presence-reference-profile.md) |
| `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` | `selta.evidence.implementation/candidate-1` | [docs/19-presence-reference-profile.md](../../../docs/19-presence-reference-profile.md) as the specification oracle, not native code |

The evaluation-profile identity is embedded in the environment and is derived
from its declared fields; it has no separate source artifact.

## Schema-source artifacts

Every identity below uses
`selta.evidence.schema-source/candidate-1` over the canonical admitted-`Node`
projection in document 16.

| Schema-source identity | Repository artifact |
|---|---|
| `sha256:c1baacc5c17045615bda704cc36e1bbb8ee17302c5e72f88bcc4b3185d822f41` | [acquisition.schema.json](../../../schemas/evidence/candidate-1/acquisition.schema.json) |
| `sha256:cd364c5c485b3765e5949638aaf4c14375bef9acb23cda1730729244e6d08483` | [basis.schema.json](../../../schemas/evidence/candidate-1/basis.schema.json) |
| `sha256:8fc79564ad3d4d2cb59287fbebb86951796bd9ea272d4570bed01706ddcff545` | [claim.schema.json](../../../schemas/evidence/candidate-1/claim.schema.json) |
| `sha256:539517aa2ea4a3661a08a00ae7bb925aeca839a4cc7a77ad6c7850c34f88bada` | [environment.schema.json](../../../schemas/evidence/candidate-1/environment.schema.json) |
| `sha256:f98ca98e741a7c5ea00a42f45631b245d60d689804b039998f2b5ddbedfd1288` | [graph.schema.json](../../../schemas/evidence/candidate-1/graph.schema.json) |
| `sha256:26d81af6fb32302ffeb0c8fe7ff7415ba1e3b675d6faab3016aec843ae3029ef` | [outcome.schema.json](../../../schemas/evidence/candidate-1/outcome.schema.json) |
| `sha256:b774ab3e3d199583167d77c770b3172899c28a352b6121a90c8b522c3ce5f14d` | [package.schema.json](../../../schemas/evidence/candidate-1/package.schema.json) |
| `sha256:d4bb1f321b19d352562cc09214630a35d7437a060ad97ea361b4cff967c65444` | [assumption-rule-input.schema.json](schemas/assumption-rule-input.schema.json) |
| `sha256:b0f8c278dc5c185a06930cec44652fc093ee21a415f7b3d8e80befb3da0e62dc` | [assumption-rule-output.schema.json](schemas/assumption-rule-output.schema.json) |
| `sha256:f31243a2ac7d389e726b0eaa97a7a376027bb0658ca7a408625e104d6cf73f8e` | [assumption-rule-parameters.schema.json](schemas/assumption-rule-parameters.schema.json) |
| `sha256:20df89cad17e11996bea9e9bb791cc5961bdf2005ca2d579bc2fdf4372e17632` | [claim-theory-input.schema.json](schemas/claim-theory-input.schema.json) |
| `sha256:0a3625fa89c3ccb6a54575ea2642ba4822c89c72e5fb53b47859cdc7fc37e40a` | [claim-theory-output.schema.json](schemas/claim-theory-output.schema.json) |
| `sha256:4c8c916e3211b3893744e001d2744208f193a0477721670cd5f89eb8abccc01e` | [interpreter-input.schema.json](schemas/interpreter-input.schema.json) |
| `sha256:43caa030873a40106f7b52cb26e97a0eb7190d56b76a1d158b59d86e2b474dbb` | [interpreter-output.schema.json](schemas/interpreter-output.schema.json) |
| `sha256:7d9f1696eacdc7ad2fa21bd54138cc0130014a5dd8794ff6eb7dfb4e8760db22` | [label.schema.json](schemas/label.schema.json) |
| `sha256:747792fca7a28e3727a50af30affe013babc368b1ab82e41edd5ab6afb34fd95` | [projection-input.schema.json](schemas/projection-input.schema.json) |
| `sha256:d879c006ea0a0c92b4e7014a8bdc2066c866c74e4511038fdb9ccb4168b16366` | [projection-output.schema.json](schemas/projection-output.schema.json) |
| `sha256:898e7d7049ae2c584c7dc1f09c7ed48a5e25c4969c77900e9bc49808aa1b1b5a` | [projection-parameters.schema.json](schemas/projection-parameters.schema.json) |
| `sha256:e59826490df11fc4929dec6dbd3aa6fb531a2877518124b59ae6771bbd897fd7` | [state.schema.json](schemas/state.schema.json) |
| `sha256:13552c5c23c91e20e8f8a526250eed1da10df6477f799842a84b30c07921100a` | [text.schema.json](schemas/text.schema.json) |
| `sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4` | [unit.schema.json](schemas/unit.schema.json) and [non_empty.schema.json](builtin-config-schemas/non_empty.schema.json) |
| `sha256:b4bcd765190f693b91a746777efc112d62fe033d6164c3abfe20463de353738a` | [len.schema.json](builtin-config-schemas/len.schema.json) |
| `sha256:a2e55f25bb787135570e5f683925a0736a2c9a2ab906ab8a5155c84a773b4038` | [one_of.schema.json](builtin-config-schemas/one_of.schema.json) |
| `sha256:7e4a048ab346b9bb188e72c9257f9b2089efe875558388c775eec77c4964a3f1` | [range.schema.json](builtin-config-schemas/range.schema.json) |
| `sha256:79bce13a82660433867f9ad6c0b48557125475331c4e145ba2965cf556c04510` | [regex.schema.json](builtin-config-schemas/regex.schema.json) |

The unit and `non_empty` configuration schemas intentionally share an identity:
their admitted canonical `Node` projections are equal. Raw formatting and file
purpose are not part of schema-source identity.
