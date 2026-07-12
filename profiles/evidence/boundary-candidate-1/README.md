# Boundary profile candidate 1

> **Status: S2 conformance boundary only.** This profile is neither an
> application schema recommendation nor a product or platform-support claim.

This environment exists only to test candidate-global boundaries that must run
after stable contract admission. Its one additional contract admits an open
application object whose member names and values are intentionally
unconstrained by Selta. This permits held-out cases for:

- candidate-wide SafeInt canonicality at an exact application-value occurrence;
- RFC 6901 pointer normalization at the 2,048 UTF-8-byte ceiling; and
- deduplication after two long pointers normalize to one ancestor.

Stable contract admission accepting a member does not make that member a
candidate SafeInt or relax candidate identity rules. The assessment layer
remains responsible for its global canonicality checks.

## Exact extension

The environment is the sealed presence environment with exactly one change:
the application-record contract below is inserted into the canonical contract
set. The evaluation profile, every existing contract, all four semantic
descriptors, their specifications, and their accepted implementations are
unchanged. No new semantic relation or numbered specification is introduced.

| Artifact | Identity |
|---|---|
| Environment | `sha256:5059697608b5bb6669cf93122a7c360ac18b68cc2e8612236e51a0634387b566` |
| Application-record schema source | `sha256:d85f64944d23b45d0178e7a9ebf79842974350bebafcb7bdd386f5bf5fb640c7` |
| Application-record contract | `sha256:c8258a3588beb32e5acecea414c2f8cbe086c13fdc04715236da6086b6017b89` |
| Reused evaluation profile | `sha256:f65b12e185f2ed2d619cb04cd2f9b52f8a3be44b9a8a67a860988863ab6c3a30` |
| Reused presence environment | `sha256:0c153635b88d6dee3f1301f1a46b20d1a4439d5f7b755fe276e8d93a8ccdf8a3` |

The complete resolver closure is recorded in [ARTIFACTS.md](ARTIFACTS.md).
Fixtures remain a separate conformance-corpus slice.

## Boundary discipline

The open schema grants no semantic operation, authority, interpretation,
projection, persistence, native access, or confidentiality property. It merely
prevents stable Selta verification from pre-empting the later candidate-global
boundary being tested. All artifacts remain path-neutral and operating-system
independent.
