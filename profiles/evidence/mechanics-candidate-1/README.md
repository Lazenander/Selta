# Mechanics profile candidate 1

This directory is the immutable environment artifact for the mechanics
conformance profile in
[document 22](../../../docs/22-mechanics-conformance-profile.md).
It is a specification oracle and conformance-corpus root, not an application
policy, shipped runtime, or platform-support claim. The complete mechanics
vectors and semantic-call ledgers are indexed in
[fixtures/README.md](fixtures/README.md).

The environment reuses contracts and schema-source identities from the core
and presence artifact trees; it does not copy their source files. It reuses
the exact claim-theory, interpreter, and projection descriptors and deliberately
omits the presence explicit-assumption rule. The nine files under `schemas/`
are its only additional Selta schema sources.

## Root identities

| Artifact | Identity |
|---|---|
| Environment | `sha256:3cc649a47f2ff7f57d1f55ea8a22623bbe8affc9bd551eb1106212ce84473d18` |
| Language source (document 15) | `sha256:71806dd912b31da7bbd8c03365654d28d6823715d3b8ca1c17b87b0c382fe0e5` |
| Core-semantics source (document 16) | `sha256:87c2564bfa13553c9dddb2d3726203239ad1aebec5f12b3cf96a9a52d32536d0` |
| Error-catalog source (document 20) | `sha256:923879b5172a4b0f35ce37aeedaece6227716614dba1581e6635943a3e0dd186` |
| Evaluation profile | `sha256:f65b12e185f2ed2d619cb04cd2f9b52f8a3be44b9a8a67a860988863ab6c3a30` |
| Presence specification (document 19) | `sha256:f08f66de43aceef481f21bb4b556ab00eee85584885e077c01c44fc09aa36b6c` |
| Mechanics specification (document 22) | `sha256:3b887f2fb911feba7fa584ee5da62a4011dc4ce25869901965ace8d192a52ab1` |
| Presence specification-oracle implementation | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |
| Mechanics specification-oracle implementation | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |

The exact document-22 bytes serve both as semantic specification and as the
manual specification-oracle implementation, under the distinct domain tags in
document 16. The latter is not native executable code. The same separation is
retained for the three reused document-19 relations.

## Schema-source identities

Every identity is computed from stable Selta's admitted canonical `Node`
projection, not from raw JSON bytes. The unit and `non_empty` configuration
schemas intentionally share an identity because their admitted projections are
equal.

| Schema role | Schema-source identity | Repository artifact |
|---|---|---|
| Core acquisition | `sha256:c1baacc5c17045615bda704cc36e1bbb8ee17302c5e72f88bcc4b3185d822f41` | [acquisition.schema.json](../../../schemas/evidence/candidate-1/acquisition.schema.json) |
| Core assessment basis | `sha256:cd364c5c485b3765e5949638aaf4c14375bef9acb23cda1730729244e6d08483` | [basis.schema.json](../../../schemas/evidence/candidate-1/basis.schema.json) |
| Core claim | `sha256:8fc79564ad3d4d2cb59287fbebb86951796bd9ea272d4570bed01706ddcff545` | [claim.schema.json](../../../schemas/evidence/candidate-1/claim.schema.json) |
| Core semantic environment | `sha256:539517aa2ea4a3661a08a00ae7bb925aeca839a4cc7a77ad6c7850c34f88bada` | [environment.schema.json](../../../schemas/evidence/candidate-1/environment.schema.json) |
| Core evidence graph | `sha256:f98ca98e741a7c5ea00a42f45631b245d60d689804b039998f2b5ddbedfd1288` | [graph.schema.json](../../../schemas/evidence/candidate-1/graph.schema.json) |
| Core assessment outcome | `sha256:26d81af6fb32302ffeb0c8fe7ff7415ba1e3b675d6faab3016aec843ae3029ef` | [outcome.schema.json](../../../schemas/evidence/candidate-1/outcome.schema.json) |
| Core package wrapper | `sha256:b774ab3e3d199583167d77c770b3172899c28a352b6121a90c8b522c3ce5f14d` | [package.schema.json](../../../schemas/evidence/candidate-1/package.schema.json) |
| Presence unit | `sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4` | [unit.schema.json](../presence-candidate-1/schemas/unit.schema.json) |
| Presence non-empty text | `sha256:13552c5c23c91e20e8f8a526250eed1da10df6477f799842a84b30c07921100a` | [text.schema.json](../presence-candidate-1/schemas/text.schema.json) |
| Presence label | `sha256:7d9f1696eacdc7ad2fa21bd54138cc0130014a5dd8794ff6eb7dfb4e8760db22` | [label.schema.json](../presence-candidate-1/schemas/label.schema.json) |
| Presence claim-theory input | `sha256:20df89cad17e11996bea9e9bb791cc5961bdf2005ca2d579bc2fdf4372e17632` | [claim-theory-input.schema.json](../presence-candidate-1/schemas/claim-theory-input.schema.json) |
| Presence claim-theory output | `sha256:0a3625fa89c3ccb6a54575ea2642ba4822c89c72e5fb53b47859cdc7fc37e40a` | [claim-theory-output.schema.json](../presence-candidate-1/schemas/claim-theory-output.schema.json) |
| Presence interpreter input | `sha256:4c8c916e3211b3893744e001d2744208f193a0477721670cd5f89eb8abccc01e` | [interpreter-input.schema.json](../presence-candidate-1/schemas/interpreter-input.schema.json) |
| Presence interpreter output | `sha256:43caa030873a40106f7b52cb26e97a0eb7190d56b76a1d158b59d86e2b474dbb` | [interpreter-output.schema.json](../presence-candidate-1/schemas/interpreter-output.schema.json) |
| Presence state | `sha256:e59826490df11fc4929dec6dbd3aa6fb531a2877518124b59ae6771bbd897fd7` | [state.schema.json](../presence-candidate-1/schemas/state.schema.json) |
| Presence projection input | `sha256:747792fca7a28e3727a50af30affe013babc368b1ab82e41edd5ab6afb34fd95` | [projection-input.schema.json](../presence-candidate-1/schemas/projection-input.schema.json) |
| Presence projection output | `sha256:d879c006ea0a0c92b4e7014a8bdc2066c866c74e4511038fdb9ccb4168b16366` | [projection-output.schema.json](../presence-candidate-1/schemas/projection-output.schema.json) |
| Presence projection parameters | `sha256:898e7d7049ae2c584c7dc1f09c7ed48a5e25c4969c77900e9bc49808aa1b1b5a` | [projection-parameters.schema.json](../presence-candidate-1/schemas/projection-parameters.schema.json) |
| Presence EvidenceResult | `sha256:b0f8c278dc5c185a06930cec44652fc093ee21a415f7b3d8e80befb3da0e62dc` | [assumption-rule-output.schema.json](../presence-candidate-1/schemas/assumption-rule-output.schema.json) |
| Mechanics policy input | `sha256:e0944cb70243269855c0024623550df311b6bb6f8b0ab1c8f2c41e6fb567946e` | [policy-input.schema.json](schemas/policy-input.schema.json) |
| Mechanics attempt proposal | `sha256:ef3010d0e3fe7f37ba227ccb952a3756fe45ad2e45663c5d372be165b41a241d` | [attempt-proposal.schema.json](schemas/attempt-proposal.schema.json) |
| Mechanics disposition proposal | `sha256:036d87923ae1921bbfc3c33c2899d3efbb793d1dbafe85af0552cfe0e5d81adf` | [disposition-proposal.schema.json](schemas/disposition-proposal.schema.json) |
| Mechanics stop transition | `sha256:3e3af37163b5843fefde7d8e9a75e00b19b333402bf6b206a6da29c6df298617` | [stop-transition.schema.json](schemas/stop-transition.schema.json) |
| Mechanics attestation input | `sha256:8a8148864234e18437cea1a98d1809777af685099161326ad7ee1d4a4ee78bf9` | [attestation-input.schema.json](schemas/attestation-input.schema.json) |
| Mechanics attestation result | `sha256:20a282b25c0af5c367d95cf9f286171896d6df19dab64b0e24bcf762138d54ba` | [attestation-result.schema.json](schemas/attestation-result.schema.json) |
| Mechanics extractor input | `sha256:719ead0360eb02082e1e0a9c85ca8f02ce026aaba5a6323e9864ddcef7431331` | [extractor-input.schema.json](schemas/extractor-input.schema.json) |
| Mechanics operational-rule input | `sha256:e74e6d7d77c75ea12ae7854754d3c13324d756d9fd4ef0ac84299034037a5d49` | [operational-rule-input.schema.json](schemas/operational-rule-input.schema.json) |
| Mechanics forwarding-rule input | `sha256:97fb35fb68b8a7305b56acc4c4a82abfea5a8e2967fd6d0a03251e2ac94eff0b` | [forwarding-rule-input.schema.json](schemas/forwarding-rule-input.schema.json) |
| Pure builtin len configuration | `sha256:b4bcd765190f693b91a746777efc112d62fe033d6164c3abfe20463de353738a` | [len.schema.json](../presence-candidate-1/builtin-config-schemas/len.schema.json) |
| Pure builtin non_empty configuration | `sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4` | [non_empty.schema.json](../presence-candidate-1/builtin-config-schemas/non_empty.schema.json) |
| Pure builtin one_of configuration | `sha256:a2e55f25bb787135570e5f683925a0736a2c9a2ab906ab8a5155c84a773b4038` | [one_of.schema.json](../presence-candidate-1/builtin-config-schemas/one_of.schema.json) |
| Pure builtin range configuration | `sha256:7e4a048ab346b9bb188e72c9257f9b2089efe875558388c775eec77c4964a3f1` | [range.schema.json](../presence-candidate-1/builtin-config-schemas/range.schema.json) |
| Pure builtin regex configuration | `sha256:79bce13a82660433867f9ad6c0b48557125475331c4e145ba2965cf556c04510` | [regex.schema.json](../presence-candidate-1/builtin-config-schemas/regex.schema.json) |

## Contracts

The first nineteen bindings reuse exact core or presence identities. The final
nine bind the sealed mechanics schema projections to the same evaluation
profile.

| Contract | ID | Schema source | Origin |
|---|---|---|---|
| Acquisition | `sha256:febf5ad5f7f06e0a014163c97de740272ce0229c8df949a4c553a84ce849cf91` | `sha256:c1baacc5c17045615bda704cc36e1bbb8ee17302c5e72f88bcc4b3185d822f41` | reused |
| Assessment basis | `sha256:938e5e124cb9648bb532949ff8588edeef0570eee18f6c7bfaf579af17a352a0` | `sha256:cd364c5c485b3765e5949638aaf4c14375bef9acb23cda1730729244e6d08483` | reused |
| Claim | `sha256:751cc09329f4a485fce9683a29ea5432786cf22b43df9cfd4b3900a326594239` | `sha256:8fc79564ad3d4d2cb59287fbebb86951796bd9ea272d4570bed01706ddcff545` | reused |
| Semantic environment | `sha256:493cb66bfa0539ef191ca326833e0fa0921e4e078001123bbbf8b0e1ca163661` | `sha256:539517aa2ea4a3661a08a00ae7bb925aeca839a4cc7a77ad6c7850c34f88bada` | reused |
| Evidence graph | `sha256:563110eb157d8aae350362c6529fec57bb18907834e3c28f6e0d068477ca177c` | `sha256:f98ca98e741a7c5ea00a42f45631b245d60d689804b039998f2b5ddbedfd1288` | reused |
| Assessment outcome | `sha256:86ed6b6f38c1b1a1d6bc5f09e2997a0df9e0f1e8554624372031034f17ebf7fb` | `sha256:26d81af6fb32302ffeb0c8fe7ff7415ba1e3b675d6faab3016aec843ae3029ef` | reused |
| Package wrapper | `sha256:3d80c13453dcb20e49ea8ec3cb9af2a588d9519b0599c4b0baca8625a8b6d73b` | `sha256:b774ab3e3d199583167d77c770b3172899c28a352b6121a90c8b522c3ce5f14d` | reused |
| Unit parameters | `sha256:060a69f7418ea07c1cf8b003b2d97b823cac675463855dc5c6bbaf3ea5f42971` | `sha256:0a823964c6368f09bce16c1d2400d4a86d25e0b005f7c5604a42e551dd168df4` | reused |
| Non-empty text | `sha256:a587092a0074c9b57a2a22225b99fb20cb6fd0ba69ea27156cab7b8b1ca6ed21` | `sha256:13552c5c23c91e20e8f8a526250eed1da10df6477f799842a84b30c07921100a` | reused |
| Presence label | `sha256:45a97f98cb6fe6d8e07bf2d0e9d820e604c428170b279199b99b1f0563586322` | `sha256:7d9f1696eacdc7ad2fa21bd54138cc0130014a5dd8794ff6eb7dfb4e8760db22` | reused |
| Claim-theory input | `sha256:6ae165902d9c0c488caffb1c65ac8c43b143ec509df271d9b9472d9701865c4a` | `sha256:20df89cad17e11996bea9e9bb791cc5961bdf2005ca2d579bc2fdf4372e17632` | reused |
| Claim-theory output | `sha256:089d28ee6d1825a8b6a9c49f75edc590868e9a5ad7ff34a47aa29a1a2389952c` | `sha256:0a3625fa89c3ccb6a54575ea2642ba4822c89c72e5fb53b47859cdc7fc37e40a` | reused |
| Interpreter input | `sha256:9e1bf91eb51623f3baa9bbdfc45a9626a576dddeadd23f7b35b52405a30a752f` | `sha256:4c8c916e3211b3893744e001d2744208f193a0477721670cd5f89eb8abccc01e` | reused |
| Interpreter output | `sha256:c20a0dc0636103a270f1ea6e704348ecd2152ef64a2286b956bfc5cd7f6c6291` | `sha256:43caa030873a40106f7b52cb26e97a0eb7190d56b76a1d158b59d86e2b474dbb` | reused |
| Presence state | `sha256:78295e80afc955a2c6450d677b024e62cb63b7d1eb4759be4af9c05c364b5dc0` | `sha256:e59826490df11fc4929dec6dbd3aa6fb531a2877518124b59ae6771bbd897fd7` | reused |
| Projection input | `sha256:af0330f2d92a354c762d2ca20471dcdf982cf1728241d3d72a76b1dc8ec39524` | `sha256:747792fca7a28e3727a50af30affe013babc368b1ab82e41edd5ab6afb34fd95` | reused |
| Projection output | `sha256:263d44ddc5668282c49cb1a2b825d320e6bf05ca30ecbbb087e24eefcdaafd88` | `sha256:d879c006ea0a0c92b4e7014a8bdc2066c866c74e4511038fdb9ccb4168b16366` | reused |
| Projection parameters | `sha256:a7d852fbeb1af4bf3c86ca82311dd7b1ee47470f44e72c0296e5735bd449f337` | `sha256:898e7d7049ae2c584c7dc1f09c7ed48a5e25c4969c77900e9bc49808aa1b1b5a` | reused |
| EvidenceResult | `sha256:8062ecad27e59838239b04878e2ee9fcbfad53479484334b9b49f1e1d4bfa8b3` | `sha256:b0f8c278dc5c185a06930cec44652fc093ee21a415f7b3d8e80befb3da0e62dc` | reused |
| Policy input | `sha256:2a414245b0717d0321687dc7397c6bc2a533555d18d46fade0dd5044ae6b5413` | `sha256:e0944cb70243269855c0024623550df311b6bb6f8b0ab1c8f2c41e6fb567946e` | mechanics |
| Attempt proposal | `sha256:70c179ed0c3bfd2d78f6822ce639ae4cea640ef1fa98682e651b760487166b85` | `sha256:ef3010d0e3fe7f37ba227ccb952a3756fe45ad2e45663c5d372be165b41a241d` | mechanics |
| Disposition proposal | `sha256:0cf27abcd4f021ad46c99079f5c9b3984c4087ff0314c05ec49f9356fb1f2b62` | `sha256:036d87923ae1921bbfc3c33c2899d3efbb793d1dbafe85af0552cfe0e5d81adf` | mechanics |
| Stop transition | `sha256:f78ce2264d1d6fb86e0aabd1a474d29c9eada0b8f255c2c7f38ea5d150a725ba` | `sha256:3e3af37163b5843fefde7d8e9a75e00b19b333402bf6b206a6da29c6df298617` | mechanics |
| Attestation input | `sha256:f9a9677800112e44858092a5a19dd419cd24c6798730c056348bee00f801eee9` | `sha256:8a8148864234e18437cea1a98d1809777af685099161326ad7ee1d4a4ee78bf9` | mechanics |
| Attestation result | `sha256:3f082e6ba83e0f641a95e87ddb11910cb23802f4ddbbcc7b1bb9b45d6fc6ac93` | `sha256:20a282b25c0af5c367d95cf9f286171896d6df19dab64b0e24bcf762138d54ba` | mechanics |
| Extractor input | `sha256:958aad35ecd5602c8967e0d777a2be0f69c6fd2f97220c5980804e4a1e27d452` | `sha256:719ead0360eb02082e1e0a9c85ca8f02ce026aaba5a6323e9864ddcef7431331` | mechanics |
| Operational-rule input | `sha256:41c8b3ed99f3b149f9621b9435397ecf71aa2ab483695a8988f60113f178f259` | `sha256:e74e6d7d77c75ea12ae7854754d3c13324d756d9fd4ef0ac84299034037a5d49` | mechanics |
| Forwarding-rule input | `sha256:f2062869ee3c0a02f51bf7bb8fe46bfdc01cbad5f88190b83db0702a55364109` | `sha256:97fb35fb68b8a7305b56acc4c4a82abfea5a8e2967fd6d0a03251e2ac94eff0b` | mechanics |

## Evidence semantics

The first three descriptors are byte-for-byte equal to their presence-profile
descriptors. The seven mechanics descriptors bind document 22. Operational and
forwarding rules have distinct input contracts and therefore distinct semantic
identities.

| Relation | Kind | Semantics ID | Bound oracle implementation |
|---|---|---|---|
| Exact claim theory | `claim_theory` | `sha256:e229afb0f7788286165dc081f68fe1e7e2d82d4f2b6eed88b4fe8041f62143f3` | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |
| Presence interpreter | `interpreter` | `sha256:0207e530315247b35e2593faf0ec8a2bdc8b20e406732f71cfe4882380ff8a1c` | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |
| Cautious projection | `projection` | `sha256:e409e51e75d345e931f9024f904600e0ccea7cf676738686dde9b054345ae546` | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |
| Declared proposal | `schedule` | `sha256:075dad81f7a6a53797c8cf98c7f264f3cac9b5c6606dec080703860d3e636e6f` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Declared disposition | `selection` | `sha256:fefa98044617743883bae9f14ce957c0cc0ef4abb11f016b7ad2b2d8645bafda` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Deterministic stop | `stopping` | `sha256:1a6482bcdcf21c0acb6b239ff67787f0d5a13c8059643a5575cc4cc0792244c5` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Accounting control | `attestation` | `sha256:6a25cdc3cc1f11b7e820da5583fab292975c6c58f7833057780a2beb9137f938` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Exact observation | `extractor` | `sha256:6aa2f61f16d40ef2dd513f34d6f01d1e698ef528671e0e149c688348e49239cf` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Explicit operational fact | `rule` | `sha256:a65164fc8aa89fffccb627fda5b5de126a9e0c6bbfe4d41f3b93f1b71f763076` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| One-premise forwarding | `rule` | `sha256:ba95e9a747bf00473fbb9d54092e2ae8f7e5b191ba131302853f2b169d873c07` | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |

## Verification boundary

All referenced schema sources admit through stable Selta's complete
pure-builtin registry and the environment verifies under the candidate
environment schema in strict `Input::Value` mode. Identity checks cover the
exact bytes of final documents 15, 16, 19, 20, and 22; profile, contract,
descriptor, and environment preimages; canonical UTF-8 ordering; complete
resolver closure; descriptor signatures; and duplicate identities.

The complete digest-to-path resolver inventory is
[ARTIFACTS.md](ARTIFACTS.md). Fixtures remain separate from the immutable
environment artifact and do not enter its identity.
