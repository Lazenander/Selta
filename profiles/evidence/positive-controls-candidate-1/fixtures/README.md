# Positive-controls conformance fixtures

These are the complete closed ordinary and forbidden vectors required by
document 23. They use environment `sha256:97a2795354041b74443f8686d75be998eebf563275b479d9d23a055adf92eba4`, the reused presence
and mechanics specification oracles, and positive-controls oracle
`sha256:b495d8e53cee79f5647241804f203ad7c50e0f8dd6088b5e94c3417985014ae8`.

Every ordinary input uses its complete package root as the caller-supplied
`ExpectedBasisRef`. Oracles compare the complete conformance projection.
The final column is
`documents/attempts/nodes/edges/semantic-calls/semantic-fuel`.

| Fixture | Kind | Conformance ID | Expected root | Exact resources |
| --- | --- | --- | --- | --- |
| `checked-witness-matching` | positive | `positive-controls-checked-witness-matching` | `sha256:894e174b7408be029ac1e603d441c972835cafdd799c6b4fbcb817502e35e895` | 34/1/2/1/9/16603 |
| `checked-witness-invalid-omitted` | positive | `positive-controls-checked-witness-invalid-omitted` | `sha256:dcd4a97df61783a2106bf605cd56721b49332ca543e6a6e2693302a2b1d89167` | 32/1/1/0/8/13565 |
| `checked-witness-invalid-submitted` | negative | `positive-controls-checked-witness-invalid-submitted` | `sha256:40e3fc06ced7bea3208e779bbfed45d0df9e84355b9a948a7198005abfc0b832` | 33/1/2/1/0/0 |
| `checked-witness-valid-refute` | negative | `positive-controls-checked-witness-valid-refute` | `sha256:fb44b50833ab8ee8b31e53bebfb3fb0822bcb1668b7c68558974117e1e6f90ef` | 33/1/2/1/7/13995 |
| `checked-witness-missing-checker-grant` | negative | `positive-controls-checked-witness-missing-checker-grant` | `sha256:331f74b98a2523abde31fe10c24fcd477dcb11e418820a244a3316a1fe168fee` | 33/1/2/1/0/0 |
| `checked-witness-unavailable-theory-assumption` | negative | `positive-controls-checked-witness-unavailable-theory-assumption` | `sha256:db7e4badc87bf1ccf779192bf784273d38aca6bef69e3960d52bda9651ea27ff` | 33/1/2/1/0/0 |
| `checked-witness-wrong-theory-statement` | negative | `positive-controls-checked-witness-wrong-theory-statement` | `sha256:f843e7d009ad6c7e0ed4ce6af5b81c7a16c8eb0be464ca68d61b41f04fcc15e8` | 35/1/2/1/0/0 |
| `checked-witness-wrong-theory-scope` | negative | `positive-controls-checked-witness-wrong-theory-scope` | `sha256:b4019fd7ba6d6bd1ac67e0fdcd9a342d92a9b4ea4aaec1385c4b37fa0f85973d` | 33/1/2/1/0/0 |
| `sequential-fixed-horizon` | positive | `positive-controls-sequential-fixed-horizon` | `sha256:e365982f9f9bbe26c0550c5b3c68733a5c3889c1ab031321b1d0de44a1c50acb` | 43/0/1/1/7/14570 |
| `sequential-anytime-valid` | positive | `positive-controls-sequential-anytime-valid` | `sha256:4e9317d939d9421f7532e3e02ec98a9ca81177f9cd9dbdb9b3a469996a771e84` | 43/0/1/1/7/14570 |
| `sequential-wrong-scope-contract` | negative | `positive-controls-sequential-wrong-scope-contract` | `sha256:315dfd3f80842f5c9da89ce0fb7ebd8fa03f24c3c8dbb27527f56c51d3d175a8` | 41/0/1/1/0/0 |
| `sequential-expectation-scope-mismatch` | negative | `positive-controls-sequential-expectation-scope-mismatch` | `sha256:2bd188564addc5e34c20bbd63c2b96238c39ab5557f6cc54d6cc878005aa05fd` | 43/0/1/1/0/0 |
| `sequential-checkpoint-mismatch` | negative | `positive-controls-sequential-checkpoint-mismatch` | `sha256:6717cbd6304a697cc8b795e3be06864f594d91ef001274b8fe3c83044c6b41da` | 43/0/1/1/0/0 |
| `sequential-missing-recorder-grant` | negative | `positive-controls-sequential-missing-recorder-grant` | `sha256:4b6526603faa3f37381899a32a5280fd43d44c2f5f67557ce4890fa87dc7f47f` | 42/0/1/1/0/0 |
| `sequential-missing-attestation-grant` | negative | `positive-controls-sequential-missing-attestation-grant` | `sha256:f6e6ad101874e2f613379bbd27b0dbf935368b5dba76585f3fc85247c594f66a` | 42/0/1/1/0/0 |
| `sequential-rejected-result` | negative | `positive-controls-sequential-rejected-result` | `sha256:ed59c2b6ee1351a1e406ac4712cda627ed682be92d7b26febcd1829986884888` | 42/0/1/1/4/6449 |
| `sequential-unavailable-assumption` | negative | `positive-controls-sequential-unavailable-assumption` | `sha256:6dbf611302c9d1bf609479b5ca55c228c75ec29d7057e6388ea4b3eb717ec33f` | 40/0/1/1/0/0 |
| `sequential-omitted-assumption` | negative | `positive-controls-sequential-omitted-assumption` | `sha256:d5be7e1c4af92d1b6b5f58e8688282ecad64c457a5fa3c633fcbb330dd9ecc69` | 42/0/1/1/0/0 |
| `sequential-duplicate-role-id` | negative | `positive-controls-sequential-duplicate-role-id` | `sha256:455fa9e0009706aa8b3a174470abaa2c0d62cb9c3f015ba9bf405e38aa8a1a4e` | 42/0/1/1/0/0 |
| `sequential-role-mismatch` | negative | `positive-controls-sequential-role-mismatch` | `sha256:0fbac0a6548e6d186208130342cf62988438d78dd6b6e6d675a39e070d28b7bc` | 42/0/1/1/0/0 |
| `sequential-statement-contract-mismatch` | negative | `positive-controls-sequential-statement-contract-mismatch` | `sha256:1afbe8f3d0338cb8fd882da2bb80c34db1aed293a8e8bab5fcbb9bbdd48e66fd` | 41/0/1/1/0/0 |
| `sequential-assumption-scope-mismatch` | negative | `positive-controls-sequential-assumption-scope-mismatch` | `sha256:fda1d40de66537ac543712cec9bf48e03c6fadcb90ea0a5840163ea0c9248e8c` | 43/0/1/1/0/0 |
| `sequential-substantive-target-binding` | negative | `positive-controls-sequential-substantive-target-binding` | `sha256:ddefcad2ab516a5809789fa34039cde48bedaa0c21f95edc32fa080cac5c58a0` | 42/0/1/1/0/0 |
| `sequential-substantive-target-output` | negative | `positive-controls-sequential-substantive-target-output` | `sha256:fe5cd3aeaf7b54d52325d96fe7c0fa5a363743ed745042686300530024c2ab5a` | 42/0/1/1/5/10860 |
| `sequential-missing-recovery-grant` | negative | `positive-controls-sequential-missing-recovery-grant` | `sha256:0ecd80532b9907cb7ea99323181ec2bd5a4863887fd91aaac2d96be2a80e14ce` | 42/0/1/1/0/0 |

The two sequential positive cases establish only recoverability of seven
caller-pinned declarations. Their substantive statistical target is exactly
`neither`; no fixture, state, label, closure, or prose asserts statistical
assurance. The invalid checked witness likewise never creates refutation.

[calls/README.md](calls/README.md) records every actual dispatch in global
schedule order. [forbidden/README.md](forbidden/README.md) records shape-valid
dependency falsifiers which are not assessment inputs.
