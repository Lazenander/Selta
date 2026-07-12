# Mechanics candidate artifact resolver

This is the complete digest-to-path inventory needed to admit
`environment.json` without searching the source tree. Paths are repository
locations, not identity preimages. Digests and domain tags are normative;
locations are replaceable only by content with the same identity.

## Root, specification, and implementation artifacts

| Identity | Domain | Repository artifact |
|---|---|---|
| `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` | `selta.evidence.implementation/candidate-1` | [22-mechanics-conformance-profile.md](../../../docs/22-mechanics-conformance-profile.md) as the mechanics specification oracle |
| `sha256:3b887f2fb911feba7fa584ee5da62a4011dc4ce25869901965ace8d192a52ab1` | `selta.evidence.semantic-specification/candidate-1` | [22-mechanics-conformance-profile.md](../../../docs/22-mechanics-conformance-profile.md) for mechanics semantics |
| `sha256:3cc649a47f2ff7f57d1f55ea8a22623bbe8affc9bd551eb1106212ce84473d18` | `selta.evidence.environment/candidate-1` | [environment.json](environment.json) |
| `sha256:71806dd912b31da7bbd8c03365654d28d6823715d3b8ca1c17b87b0c382fe0e5` | `selta.evidence.language-specification/candidate-1` | [15-evidence-language.md](../../../docs/15-evidence-language.md) |
| `sha256:87c2564bfa13553c9dddb2d3726203239ad1aebec5f12b3cf96a9a52d32536d0` | `selta.evidence.core-semantics/candidate-1` | [16-evidence-semantics.md](../../../docs/16-evidence-semantics.md) |
| `sha256:923879b5172a4b0f35ce37aeedaece6227716614dba1581e6635943a3e0dd186` | `selta.evidence.error-catalog/candidate-1` | [20-evidence-error-catalog.md](../../../docs/20-evidence-error-catalog.md) |
| `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` | `selta.evidence.implementation/candidate-1` | [19-presence-reference-profile.md](../../../docs/19-presence-reference-profile.md) as the presence specification oracle |
| `sha256:f08f66de43aceef481f21bb4b556ab00eee85584885e077c01c44fc09aa36b6c` | `selta.evidence.semantic-specification/candidate-1` | [19-presence-reference-profile.md](../../../docs/19-presence-reference-profile.md) for reused semantics |

The evaluation-profile identity is embedded in the environment and has no
separate source artifact. Implementation artifacts are supplied by the host
registry rather than referenced by the environment manifest.

## Schema-source artifacts

Every identity below uses `selta.evidence.schema-source/candidate-1` over stable Selta's canonical
admitted-`Node` projection. Multiple paths on one row have exactly equal
projections and therefore one resolver identity.

| Schema-source identity | Repository artifact |
|---|---|
| `sha256:036d87923ae1921bbfc3c33c2899d3efbb793d1dbafe85af0552cfe0e5d81adf` | [disposition-proposal.schema.json](schemas/disposition-proposal.schema.json) |
| `sha256:0a3625fa89c3ccb6a54575ea2642ba4822c89c72e5fb53b47859cdc7fc37e40a` | [claim-theory-output.schema.json](../presence-candidate-1/schemas/claim-theory-output.schema.json) |
| `sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4` | [unit.schema.json](../presence-candidate-1/schemas/unit.schema.json) and [non_empty.schema.json](../presence-candidate-1/builtin-config-schemas/non_empty.schema.json) |
| `sha256:13552c5c23c91e20e8f8a526250eed1da10df6477f799842a84b30c07921100a` | [text.schema.json](../presence-candidate-1/schemas/text.schema.json) |
| `sha256:20a282b25c0af5c367d95cf9f286171896d6df19dab64b0e24bcf762138d54ba` | [attestation-result.schema.json](schemas/attestation-result.schema.json) |
| `sha256:20df89cad17e11996bea9e9bb791cc5961bdf2005ca2d579bc2fdf4372e17632` | [claim-theory-input.schema.json](../presence-candidate-1/schemas/claim-theory-input.schema.json) |
| `sha256:26d81af6fb32302ffeb0c8fe7ff7415ba1e3b675d6faab3016aec843ae3029ef` | [outcome.schema.json](../../../schemas/evidence/candidate-1/outcome.schema.json) |
| `sha256:3e3af37163b5843fefde7d8e9a75e00b19b333402bf6b206a6da29c6df298617` | [stop-transition.schema.json](schemas/stop-transition.schema.json) |
| `sha256:43caa030873a40106f7b52cb26e97a0eb7190d56b76a1d158b59d86e2b474dbb` | [interpreter-output.schema.json](../presence-candidate-1/schemas/interpreter-output.schema.json) |
| `sha256:4c8c916e3211b3893744e001d2744208f193a0477721670cd5f89eb8abccc01e` | [interpreter-input.schema.json](../presence-candidate-1/schemas/interpreter-input.schema.json) |
| `sha256:539517aa2ea4a3661a08a00ae7bb925aeca839a4cc7a77ad6c7850c34f88bada` | [environment.schema.json](../../../schemas/evidence/candidate-1/environment.schema.json) |
| `sha256:719ead0360eb02082e1e0a9c85ca8f02ce026aaba5a6323e9864ddcef7431331` | [extractor-input.schema.json](schemas/extractor-input.schema.json) |
| `sha256:747792fca7a28e3727a50af30affe013babc368b1ab82e41edd5ab6afb34fd95` | [projection-input.schema.json](../presence-candidate-1/schemas/projection-input.schema.json) |
| `sha256:79bce13a82660433867f9ad6c0b48557125475331c4e145ba2965cf556c04510` | [regex.schema.json](../presence-candidate-1/builtin-config-schemas/regex.schema.json) |
| `sha256:7d9f1696eacdc7ad2fa21bd54138cc0130014a5dd8794ff6eb7dfb4e8760db22` | [label.schema.json](../presence-candidate-1/schemas/label.schema.json) |
| `sha256:7e4a048ab346b9bb188e72c9257f9b2089efe875558388c775eec77c4964a3f1` | [range.schema.json](../presence-candidate-1/builtin-config-schemas/range.schema.json) |
| `sha256:898e7d7049ae2c584c7dc1f09c7ed48a5e25c4969c77900e9bc49808aa1b1b5a` | [projection-parameters.schema.json](../presence-candidate-1/schemas/projection-parameters.schema.json) |
| `sha256:8a8148864234e18437cea1a98d1809777af685099161326ad7ee1d4a4ee78bf9` | [attestation-input.schema.json](schemas/attestation-input.schema.json) |
| `sha256:8fc79564ad3d4d2cb59287fbebb86951796bd9ea272d4570bed01706ddcff545` | [claim.schema.json](../../../schemas/evidence/candidate-1/claim.schema.json) |
| `sha256:97fb35fb68b8a7305b56acc4c4a82abfea5a8e2967fd6d0a03251e2ac94eff0b` | [forwarding-rule-input.schema.json](schemas/forwarding-rule-input.schema.json) |
| `sha256:a2e55f25bb787135570e5f683925a0736a2c9a2ab906ab8a5155c84a773b4038` | [one_of.schema.json](../presence-candidate-1/builtin-config-schemas/one_of.schema.json) |
| `sha256:b0f8c278dc5c185a06930cec44652fc093ee21a415f7b3d8e80befb3da0e62dc` | [assumption-rule-output.schema.json](../presence-candidate-1/schemas/assumption-rule-output.schema.json) |
| `sha256:b4bcd765190f693b91a746777efc112d62fe033d6164c3abfe20463de353738a` | [len.schema.json](../presence-candidate-1/builtin-config-schemas/len.schema.json) |
| `sha256:b774ab3e3d199583167d77c770b3172899c28a352b6121a90c8b522c3ce5f14d` | [package.schema.json](../../../schemas/evidence/candidate-1/package.schema.json) |
| `sha256:c1baacc5c17045615bda704cc36e1bbb8ee17302c5e72f88bcc4b3185d822f41` | [acquisition.schema.json](../../../schemas/evidence/candidate-1/acquisition.schema.json) |
| `sha256:cd364c5c485b3765e5949638aaf4c14375bef9acb23cda1730729244e6d08483` | [basis.schema.json](../../../schemas/evidence/candidate-1/basis.schema.json) |
| `sha256:d879c006ea0a0c92b4e7014a8bdc2066c866c74e4511038fdb9ccb4168b16366` | [projection-output.schema.json](../presence-candidate-1/schemas/projection-output.schema.json) |
| `sha256:e0944cb70243269855c0024623550df311b6bb6f8b0ab1c8f2c41e6fb567946e` | [policy-input.schema.json](schemas/policy-input.schema.json) |
| `sha256:e59826490df11fc4929dec6dbd3aa6fb531a2877518124b59ae6771bbd897fd7` | [state.schema.json](../presence-candidate-1/schemas/state.schema.json) |
| `sha256:e74e6d7d77c75ea12ae7854754d3c13324d756d9fd4ef0ac84299034037a5d49` | [operational-rule-input.schema.json](schemas/operational-rule-input.schema.json) |
| `sha256:ef3010d0e3fe7f37ba227ccb952a3756fe45ad2e45663c5d372be165b41a241d` | [attempt-proposal.schema.json](schemas/attempt-proposal.schema.json) |
| `sha256:f98ca98e741a7c5ea00a42f45631b245d60d689804b039998f2b5ddbedfd1288` | [graph.schema.json](../../../schemas/evidence/candidate-1/graph.schema.json) |

These rows close every language, core-semantics, error-catalog,
configuration-schema, contract-schema, and semantic-specification reference in
the manifest. The document-22 implementation row records the separate manual
specification-oracle binding; it is not native executable code.
