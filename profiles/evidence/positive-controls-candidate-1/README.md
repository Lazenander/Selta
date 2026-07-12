# Positive-controls profile candidate 1

This directory is the immutable artifact root for the positive-controls
conformance profile in
[document 23](../../../docs/23-positive-controls-profile.md). It is a
specification oracle and future conformance-corpus root, not an application
policy, shipped runtime, authentication claim, or statistical guarantee.

The environment is an exact strict extension of the sealed mechanics
environment. It reuses all 28 mechanics contracts, all 10 semantic descriptors,
the evaluation profile, and every core/profile artifact by identity. It adds
only eight contracts and the three descriptors specified by document 23.
No source file is copied from an earlier artifact tree.

## Root identities

| Artifact | Identity |
|---|---|
| Environment | `sha256:a7f22ab207310dd3f8f00c6203f212f3b0975f640750a4fff8a56636a5edbe19` |
| Reused mechanics environment | `sha256:3cc649a47f2ff7f57d1f55ea8a22623bbe8affc9bd551eb1106212ce84473d18` |
| Language source (document 15) | `sha256:71806dd912b31da7bbd8c03365654d28d6823715d3b8ca1c17b87b0c382fe0e5` |
| Core-semantics source (document 16) | `sha256:87c2564bfa13553c9dddb2d3726203239ad1aebec5f12b3cf96a9a52d32536d0` |
| Error-catalog source (document 20) | `sha256:923879b5172a4b0f35ce37aeedaece6227716614dba1581e6635943a3e0dd186` |
| Evaluation profile | `sha256:f65b12e185f2ed2d619cb04cd2f9b52f8a3be44b9a8a67a860988863ab6c3a30` |
| Presence specification (document 19) | `sha256:f08f66de43aceef481f21bb4b556ab00eee85584885e077c01c44fc09aa36b6c` |
| Mechanics specification (document 22) | `sha256:3b887f2fb911feba7fa584ee5da62a4011dc4ce25869901965ace8d192a52ab1` |
| Positive-controls specification (document 23) | `sha256:20f0f2c555eaa10f8059baaa0169c263eb11ff1db62322c29d73d08f2dabf48a` |
| Presence specification-oracle implementation | `sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0` |
| Mechanics specification-oracle implementation | `sha256:1138187cc0080e4074313f08eda11096c8164ac13387802e230b2ae968ec69ff` |
| Positive-controls specification-oracle implementation | `sha256:51eac5c914d8a16bebaaf7edc42c1c090b9c032b4347a304f1aa9988481fecb8` |

The exact document-23 bytes serve as semantic specification and manual
specification-oracle implementation under distinct domain tags. The oracle is
not native executable code.

## Added schema sources and contracts

The reused 28 contracts are exactly the mechanics contract set. The following
eight are the complete added set; their schema identities are computed from
stable Selta's admitted `Node` projections.

| Contract role | Contract ID | Schema-source ID | Repository artifact |
|---|---|---|---|
| Checked-witness theory | `sha256:5931e6153f2fad63cf97f68cdb241d7a2057fbf96a066f91791d6e81198355c7` | `sha256:ebc1d377e51da5629a2c2f85a5e58c0899fd88d0a1e0a96abd27dd2f66636319` | [checked-witness-theory.schema.json](schemas/checked-witness-theory.schema.json) |
| Sequential attestation input | `sha256:65f7194f22f59c32178c84725decfe8d2c1d1cdb0676d502f7d50f0bffd5e4d0` | `sha256:7416fdc9606c10bdf3baa01c1693cd91e25c9327cb756b4e122ddcb66374fb60` | [sequential-attestation-input.schema.json](schemas/sequential-attestation-input.schema.json) |
| Checked-witness rule input | `sha256:6f89d2f6e80da377c261cb1a7d5dd03c16428c5f077c4d542b1a4fc98134fd6e` | `sha256:88ed6ef01d296dabffc45f695960cc946e8360ea094932e1cf6ea60c43cb1f9c` | [checked-witness-rule-input.schema.json](schemas/checked-witness-rule-input.schema.json) |
| Sequential assumption | `sha256:88cb8666ce356c31c9820788eb44a26c3fc059f416e62ef66ce77177ceb61f2c` | `sha256:7f7a815c00941e1186f8a7dc0f98e12d81a31a8f0ec14f0250bab7e3e68818d0` | [sequential-assumption.schema.json](schemas/sequential-assumption.schema.json) |
| Sequential basis | `sha256:9c3d8414b5b66a10199a53efd30ddbd973848a75c98a783261d69d4e7728de03` | `sha256:16c4e7c7126593250473e7030129d21f8d404ec00ea535acb08aa4d335388687` | [sequential-basis.schema.json](schemas/sequential-basis.schema.json) |
| SHA-256 preimage existential | `sha256:c22e37686725b91582a4d4f20ce84367ea89f622b799ee1b109fe7fe3656a85f` | `sha256:a9177145873423b7ae47faa74a54eeb9e6171190e64645810ce795226dce580e` | [sha256-preimage-existential.schema.json](schemas/sha256-preimage-existential.schema.json) |
| Sequential recovery rule input | `sha256:e9676f7bb9f1be19d7b4a0927180dbe2b862377d39c82312a1ab3bf5418e3e7a` | `sha256:3decf3f72c426d01cd9c3e1aec4878c785f623e8c586f8edf26a54985e01ff85` | [sequential-recovery-rule-input.schema.json](schemas/sequential-recovery-rule-input.schema.json) |
| Sequential accounting expectation | `sha256:f0e46d4e4804450e0ea8268eac977b59b76758952ea80a9d430a1205e88b7644` | `sha256:13b16cc02138b704938cf94c84471604cf7c5fc2e29530bffa2090658632415b` | [sequential-accounting-expectation.schema.json](schemas/sequential-accounting-expectation.schema.json) |

## Semantic descriptors

The environment contains these 13 exact descriptors. Reused descriptors retain
their original specification sources and signatures.

| Relation | Kind | Semantics ID | Specification |
|---|---|---|---|
| Presence interpreter | `interpreter` | `sha256:0207e530315247b35e2593faf0ec8a2bdc8b20e406732f71cfe4882380ff8a1c` | presence |
| Declared proposal | `schedule` | `sha256:075dad81f7a6a53797c8cf98c7f264f3cac9b5c6606dec080703860d3e636e6f` | mechanics |
| Deterministic stop | `stopping` | `sha256:1a6482bcdcf21c0acb6b239ff67787f0d5a13c8059643a5575cc4cc0792244c5` | mechanics |
| Accounting control | `attestation` | `sha256:6a25cdc3cc1f11b7e820da5583fab292975c6c58f7833057780a2beb9137f938` | mechanics |
| Exact observation | `extractor` | `sha256:6aa2f61f16d40ef2dd513f34d6f01d1e698ef528671e0e149c688348e49239cf` | mechanics |
| Checked existential witness | `rule` | `sha256:6dccfeb3bde11ced8205aecd1bd381411e67fc46b01b51a31443c85cb9d3bf93` | positive controls |
| Explicit operational fact | `rule` | `sha256:a65164fc8aa89fffccb627fda5b5de126a9e0c6bbfe4d41f3b93f1b71f763076` | mechanics |
| Sequential accounting expectation | `attestation` | `sha256:b62447545051b10f5eb4dfd0568d165beef78aebe8579e48d88986e242387d22` | positive controls |
| One-premise forwarding | `rule` | `sha256:ba95e9a747bf00473fbb9d54092e2ae8f7e5b191ba131302853f2b169d873c07` | mechanics |
| Exact claim theory | `claim_theory` | `sha256:e229afb0f7788286165dc081f68fe1e7e2d82d4f2b6eed88b4fe8041f62143f3` | presence |
| Cautious projection | `projection` | `sha256:e409e51e75d345e931f9024f904600e0ccea7cf676738686dde9b054345ae546` | presence |
| Sequential basis recovery | `rule` | `sha256:ee2f60a9610b753587e5804f4deac01a224ddb638bbe660624ad7b8dfc86f0ed` | positive controls |
| Declared disposition | `selection` | `sha256:fefa98044617743883bae9f14ce957c0cc0ef4abb11f016b7ad2b2d8645bafda` | mechanics |

### Added signatures

| Relation | Kind | Semantics ID | Parameter contract | Input contract | Output contract |
|---|---|---|---|---|---|
| Checked existential witness | `rule` | `sha256:6dccfeb3bde11ced8205aecd1bd381411e67fc46b01b51a31443c85cb9d3bf93` | `sha256:5931e6153f2fad63cf97f68cdb241d7a2057fbf96a066f91791d6e81198355c7` | `sha256:6f89d2f6e80da377c261cb1a7d5dd03c16428c5f077c4d542b1a4fc98134fd6e` | `sha256:8062ecad27e59838239b04878e2ee9fcbfad53479484334b9b49f1e1d4bfa8b3` |
| Sequential accounting expectation | `attestation` | `sha256:b62447545051b10f5eb4dfd0568d165beef78aebe8579e48d88986e242387d22` | `sha256:f0e46d4e4804450e0ea8268eac977b59b76758952ea80a9d430a1205e88b7644` | `sha256:65f7194f22f59c32178c84725decfe8d2c1d1cdb0676d502f7d50f0bffd5e4d0` | `sha256:3f082e6ba83e0f641a95e87ddb11910cb23802f4ddbbcc7b1bb9b45d6fc6ac93` |
| Sequential basis recovery | `rule` | `sha256:ee2f60a9610b753587e5804f4deac01a224ddb638bbe660624ad7b8dfc86f0ed` | `sha256:8062ecad27e59838239b04878e2ee9fcbfad53479484334b9b49f1e1d4bfa8b3` | `sha256:e9676f7bb9f1be19d7b4a0927180dbe2b862377d39c82312a1ab3bf5418e3e7a` | `sha256:8062ecad27e59838239b04878e2ee9fcbfad53479484334b9b49f1e1d4bfa8b3` |

Both added rules declare `premise_order: "set"`. The added attestation has
no premise-order field. All three use `execution_class: "builtin_total"` and
bind the exact document-23 specification identity.

## Verification boundary

All 36 contracts and 13 descriptors are canonical, unique, and ID-sorted.
Every added schema admits through stable Selta's complete pure-builtin registry,
and the environment value verifies strictly under the candidate environment
schema. Identity checks cover the exact source bytes, admitted schema
projections, contract and descriptor preimages, environment preimage, signature
closure, and exact 28+8 / 10+3 extension deltas.

The complete digest-to-path resolver inventory is
[ARTIFACTS.md](ARTIFACTS.md). Fixtures and conformance cases remain a separate
slice.
