# Presence profile candidate 1

This directory is the immutable artifact set for
[the presence reference profile](../../../docs/19-presence-reference-profile.md).
It is a specification oracle and conformance corpus, not a shipped evidence
runtime or platform-support claim.

## Root identities

| Artifact | Identity |
|---|---|
| Environment | `sha256:0c153635b88d6dee3f1301f1a46b20d1a4439d5f7b755fe276e8d93a8ccdf8a3` |
| Language source | `sha256:71806dd912b31da7bbd8c03365654d28d6823715d3b8ca1c17b87b0c382fe0e5` |
| Core-semantics source | `sha256:87c2564bfa13553c9dddb2d3726203239ad1aebec5f12b3cf96a9a52d32536d0` |
| Error-catalog source | `sha256:923879b5172a4b0f35ce37aeedaece6227716614dba1581e6635943a3e0dd186` |
| Evaluation profile | `sha256:f65b12e185f2ed2d619cb04cd2f9b52f8a3be44b9a8a67a860988863ab6c3a30` |
| Profile specification | `sha256:f08f66de43aceef481f21bb4b556ab00eee85584885e077c01c44fc09aa36b6c` |
| Specification-oracle implementation | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |

The three core-source identities bind the exact bytes of documents 15, 16, and
20. The specification and oracle identities use different domain tags over the
same exact bytes of document 19. The oracle identity denotes manual evaluation
from that specification; it is not a compiled executable. The complete
digest-to-path resolver inventory is [ARTIFACTS.md](ARTIFACTS.md), and the
environment manifest is [environment.json](environment.json).

## Evidence semantics

| Relation | Semantics ID |
|---|---|
| Exact claim theory | `sha256:e229afb0f7788286165dc081f68fe1e7e2d82d4f2b6eed88b4fe8041f62143f3` |
| Explicit-assumption rule | `sha256:6cc57951fdeef7d63e359fa59ac7f19edf1bd8fcc034c6c8c7ac85cee9373e59` |
| Presence interpreter | `sha256:0207e530315247b35e2593faf0ec8a2bdc8b20e406732f71cfe4882380ff8a1c` |
| Cautious projection | `sha256:e409e51e75d345e931f9024f904600e0ccea7cf676738686dde9b054345ae546` |

## Contracts

| Contract | ID |
|---|---|
| Acquisition | `sha256:febf5ad5f7f06e0a014163c97de740272ce0229c8df949a4c553a84ce849cf91` |
| Assessment basis | `sha256:938e5e124cb9648bb532949ff8588edeef0570eee18f6c7bfaf579af17a352a0` |
| Claim | `sha256:751cc09329f4a485fce9683a29ea5432786cf22b43df9cfd4b3900a326594239` |
| Semantic environment | `sha256:493cb66bfa0539ef191ca326833e0fa0921e4e078001123bbbf8b0e1ca163661` |
| Evidence graph | `sha256:563110eb157d8aae350362c6529fec57bb18907834e3c28f6e0d068477ca177c` |
| Assessment outcome | `sha256:86ed6b6f38c1b1a1d6bc5f09e2997a0df9e0f1e8554624372031034f17ebf7fb` |
| Package wrapper | `sha256:3d80c13453dcb20e49ea8ec3cb9af2a588d9519b0599c4b0baca8625a8b6d73b` |
| Assumption-rule input | `sha256:80192bd03a8110215d2138790007d1b3ffac34c7c68c2768dd426849bbae360c` |
| Assumption-rule output | `sha256:8062ecad27e59838239b04878e2ee9fcbfad53479484334b9b49f1e1d4bfa8b3` |
| Assumption-rule parameters | `sha256:992ccdc58c7f52f913e18b2ca0aab8c0d8f27bf6d726aa520738dd5e9911c16b` |
| Claim-theory input | `sha256:6ae165902d9c0c488caffb1c65ac8c43b143ec509df271d9b9472d9701865c4a` |
| Claim-theory output | `sha256:089d28ee6d1825a8b6a9c49f75edc590868e9a5ad7ff34a47aa29a1a2389952c` |
| Interpreter input | `sha256:9e1bf91eb51623f3baa9bbdfc45a9626a576dddeadd23f7b35b52405a30a752f` |
| Interpreter output | `sha256:c20a0dc0636103a270f1ea6e704348ecd2152ef64a2286b956bfc5cd7f6c6291` |
| Presence label | `sha256:45a97f98cb6fe6d8e07bf2d0e9d820e604c428170b279199b99b1f0563586322` |
| Projection input | `sha256:af0330f2d92a354c762d2ca20471dcdf982cf1728241d3d72a76b1dc8ec39524` |
| Projection output | `sha256:263d44ddc5668282c49cb1a2b825d320e6bf05ca30ecbbb087e24eefcdaafd88` |
| Projection parameters | `sha256:a7d852fbeb1af4bf3c86ca82311dd7b1ee47470f44e72c0296e5735bd449f337` |
| Presence state | `sha256:78295e80afc955a2c6450d677b024e62cb63b7d1eb4759be4af9c05c364b5dc0` |
| Non-empty text | `sha256:a587092a0074c9b57a2a22225b99fb20cb6fd0ba69ea27156cab7b8b1ca6ed21` |
| Unit parameters | `sha256:060a69f7418ea07c1cf8b003b2d97b823cac675463855dc5c6bbaf3ea5f42971` |

## Verification boundary

Every schema under `schemas/` and `builtin-config-schemas/` is admitted through
stable Selta's complete pure-builtin registry with the exact limits in document
16. The environment value verifies under the candidate environment schema with
strict `Input::Value`, no cache, no settings, no monitor, and no deadline.
Relational ID validation follows the domain tags and preimages in document 16.
Every schema-source identity is also recomputed from stable Selta's admitted
`Node` projection independently of the corpus authoring generator.

The positive, negative, and forbidden vectors are indexed in
[fixtures/README.md](fixtures/README.md). Stable Selta proves their declared
shape boundary; the candidate relational and semantic expectations are the
black-box oracle for a later independent implementation.
