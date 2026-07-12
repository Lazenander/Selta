# Presence conformance fixtures

All vectors use environment
`sha256:6cdcb1b5d26f2b713a4fed5dcc5f8f6306895cacb4d733d4eb356eff61017225`
and specification oracle
`sha256:bc899c6733c11b1c452e811ef662cb7bc808e792c3c18af64fd4e6f763283ea0`.
For every `assess` case, the caller supplies the input package's complete
`root` as `ExpectedBasisRef`.

## Positive assessment vectors

| State | Basis/root digest | Generated state digest | Expected outcome/root digest |
|---|---|---|---|
| `neither` | `sha256:8362b21c1aa7bcbe120b6c17bbcbd21b9e4a34b8d13f396ef9fc4de9bd963adb` | `sha256:3dd05b8e0ef127f75acfa8a84ff0679e22e2ad5766925851c4b3d5f284193a68` | `sha256:4453da0b985f7cf72633d117c673810be65156ea85a3cb44bce55ebd053cd342` |
| `support_only` | `sha256:0e3f6d6e9ec1b1fe42eaa208e4860eed05562864f7cdf632347c3c379310b735` | `sha256:a1c6319780e8148b02fe1286753562756d9fc765aa62c072b66e119437add9af` | `sha256:38f9b9c958aa0e5ea4f4700d8203d8d9a0365c5596f1ca0e28e35ca4d770449c` |
| `refute_only` | `sha256:195ddc0e1f06b937ae0f7e5136830541476d3671ced74a820ef322bb335c3bf4` | `sha256:28d7daa3f5ab4658c72bab64294d19a3eb8e136a3daf618b5e00d7f3cca4fd13` | `sha256:2a637fdf8bf3ca5ba3570da95610db24e3169a703dcbca6828a58ff70332c9a1` |
| `both` | `sha256:42d033778eaf5a38e7a68c0cc1a5f958a6987937d05c18fe497d1cec3b1d989c` | `sha256:cb9e59bd852d4482c59cf80cf1979144cc7d345724ee45fe5f9f5468f8cf5004` | `sha256:9d0917f19d79c0c505982c04e4a2cd09d4d0894f16f4821d507d1c0e6f7a61fb` |

The `neither` trust ID is
`sha256:e6c590f38a19ee7ca079162ee390f12018463cd5849297c3f60ce86013010ee6`.
The three assumption-bearing vectors share trust ID
`sha256:e4dac2f866aee75ad73eeca1e2bf9d232c283d996d48581f105057cca9c6d8c1`.

| State | Documents | Document bytes | Nodes | Calls | Fuel | State bytes | Labels |
|---|---:|---:|---:|---:|---:|---:|---:|
| `neither` | 11 | 5,876 | 0 | 3 | 4,119 | 301 | 2 |
| `support_only` | 17 | 9,627 | 1 | 4 | 5,811 | 379 | 1 |
| `refute_only` | 17 | 9,617 | 1 | 4 | 5,808 | 378 | 1 |
| `both` | 21 | 12,724 | 2 | 5 | 7,824 | 444 | 2 |

Attempts, graph edges, and maximum fan-in are zero in all four vectors. Graph
depth is zero for `neither` and one for the other three.

Each state has an input package, complete expected outcome package, interpreter
input/output, and projection input/output. `exact-claim.*` exposes claim
canonicalization; assumption-bearing states additionally expose each rule
input/output. `rule-<n>` uses zero-based evidence-node-ID dispatch order.

## Rejection and falsification vectors

- [negative/README.md](negative/README.md) indexes six complete input/error
  package pairs with exact errors, counters, execution sets, and roots.
- [forbidden/README.md](forbidden/README.md) indexes schema-invalid and
  schema-valid outputs that an implementation must not confuse with the oracle.

Every positive and negative package wrapper and every contained typed value
passes its real stable Selta schema in strict `Input::Value` mode. Positive
outcomes additionally satisfy content identity, exact reference closure,
canonical ordering, caller-basis authority, presence semantics, dependency
functions, cautious projection, and resource formulas. The oracle implementation
ID denotes the exact bytes of document 19, not a compiled-runtime claim.
