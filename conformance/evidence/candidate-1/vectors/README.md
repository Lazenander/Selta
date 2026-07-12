# Public JCS vectors

`rfc8785.json` is an oracle-free implementer-kit artifact. It binds the two
worked examples in [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html):
primitive serialization from section 3.2.2 and UTF-16 property ordering from
section 3.2.3. A third nested value makes recursive object sorting and array
order explicit.

Each `input` is I-JSON. `expected_utf8_hex` is the lowercase hexadecimal
encoding of its exact RFC 8785 canonical UTF-8 bytes. Rows are a canonical set
ordered by `name`; the arbiter admits this record under
`jcs-vectors.schema.json`, decodes each expected byte string, and requires
byte equality with its own canonicalization.

The artifact does not contain candidate outcomes, semantic intermediates, or
withheld expectations.
