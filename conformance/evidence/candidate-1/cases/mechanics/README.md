# Mechanics conformance case records

These records bind mechanics fixtures by ordinary raw-byte SHA-256; package
bytes remain under `profiles/evidence/mechanics-candidate-1/fixtures/`, and
oracles remain separately withholdable.

For each of the 11 positive and 14 negative ordinary fixtures, the record ID
is `mechanics-<fixture-name>`. Its source is respectively
`positive/<fixture-name>.input-package.json` or
`negative/<fixture-name>.input-package.json`; the paired oracle binds the
matching expected outcome or error package and compares the complete
conformance projection. Every caller-pinned basis is the exact input-package
root.

The same directory also owns the three conformance-control records, without
renaming them:

- `mechanics-operational-forbidden-read`
- `mechanics-opaque-payload-mutant-a`
- `mechanics-opaque-payload-mutant-b`

Controls remain outside candidate package bytes, identities, resources, and
product interfaces.
