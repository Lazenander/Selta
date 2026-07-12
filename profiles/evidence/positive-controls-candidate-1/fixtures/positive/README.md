# Positive positive-controls assessments

All inputs are identity-valid, canonically ordered, document-closed, and paired
with their exact concluded package.

| Fixture | Required observation | Expected root | Exact resources |
| --- | --- | --- | --- |
| `checked-witness-matching` | one acquired UTF-8 witness matches the existential target and yields checked support | `sha256:894e174b7408be029ac1e603d441c972835cafdd799c6b4fbcb817502e35e895` | 34/1/2/1/9/16603 |
| `checked-witness-invalid-omitted` | the mismatching candidate is observed but no failed checked derivation is submitted; target remains neither | `sha256:dcd4a97df61783a2106bf605cd56721b49332ca543e6a6e2693302a2b1d89167` | 32/1/1/0/8/13565 |
| `sequential-fixed-horizon` | seven fixed-horizon declarations are recoverable; only the meta-target is support_only | `sha256:e365982f9f9bbe26c0550c5b3c68733a5c3889c1ab031321b1d0de44a1c50acb` | 43/0/1/1/7/14570 |
| `sequential-anytime-valid` | seven anytime-valid declarations are recoverable; no statistical assurance is inferred | `sha256:4e9317d939d9421f7532e3e02ec98a9ca81177f9cd9dbdb9b3a469996a771e84` | 43/0/1/1/7/14570 |

The invalid candidate in `checked-witness-invalid-omitted` is retained as an
observed response and exact observation atom, while the failed checked
derivation is absent. The existential target therefore remains `neither`.
Both sequential cases keep the substantive target at `neither`.
