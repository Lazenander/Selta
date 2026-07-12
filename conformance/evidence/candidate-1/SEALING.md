# Candidate-1 conformance sealing

> **Status: S2 conformance procedure, not candidate evidence semantics.**
> Documents 15, 16, and 20 remain the identity-bound language, judgment, and
> error definitions. These records make an evidence run reproducible; they do
> not add a product API, supported runtime, or third assessment implementation.

## Two kinds of identity

Every retained file has an ordinary raw-byte digest:

```text
bytes_sha256 = "sha256:" + lowercase_hex(SHA-256(exact file bytes))
```

Typed conformance records additionally have a semantic identity:

```text
H(tag, value) = SHA-256(UTF8(tag) || 0x00 || JCS(value))
```

The exact tags are the `revision` strings of the artifact-set, seal,
kit-index, runtime-evidence, prediction-ledger, and completion-ledger schemas.
A reference to one of those records is `{ id, bytes_sha256 }`: the first binds
admitted content, while the second binds its exact retained formatting. A
record never contains its own ID.

Candidate source references are the exception only in domain: language,
core-semantics, and error-catalog IDs retain the distinct document-16 `HB`
tags. They still carry their ordinary raw-byte digest beside that ID.

Before calculating any typed identity, the arbiter admits the record and
checks its relational canonical form. Artifact-set entries are unique and
ordered by `path`. A kit index orders contexts by `id`, each context's resolver
rows by `(digest, domain, source)`, and cases by `id`. A prediction ledger
orders runs by `case`, assigns `ordinal` equal to the zero-based array index,
and contains each required case exactly once. A completion ledger orders
evaluations by `(case, subject_rank)`, where
`model_a < model_b < arbiter`, assigns `ordinal` equal to the zero-based array
index, and contains each required pair exactly once. Freeze `kits` and
completion `predictions` contain each producer exactly once in `model_a`, then
`model_b` order. Runtime environment rows are unique and ordered by `name`;
probes are unique and ordered by `name`; argument arrays preserve command
order. A noncanonical record has no conformance identity even when its local
Selta schema admits it.

## Authored input and generated manifest

`manifest-input.json` contains only choices the arbiter cannot derive:

- bounded discovery roots and four environment paths;
- the global resolver-source path pool;
- exact raw-only artifacts;
- common public-kit inclusions, the Model-A-only stable Rust boundary, and the
  Model-B retained-runtime profile, root, and generated set path;
- the fifteen law mappings, sixteen named mappings, and forty-eight error
  classifications.

It contains no artifact hash, environment identity, resolver row, case/oracle
pair, expected path, error-case array, executable identity, or review status.
The arbiter derives those values and renders `manifest.json`; the generated
manifest is never edited manually. Its artifact entries are unique by raw
digest and carry a canonical set of zero or more Selta admission obligations.
The manifest cannot inventory itself.

“Falsifier” means a case that kills a stated nonconforming mutation. It need
not itself be an error input, and a case may legitimately occur in both the
positive and falsifier sets when its several observations distinguish both.

## Generic artifact sets

An artifact set binds exact layout as well as bytes. Entries are unique and
ASCII-lexically ordered by repository-relative `path`; equal bytes at distinct
paths remain distinct entries. Each entry must resolve beneath the set root to
one regular file. Symlinks, traversal, missing files, non-regular files, and
byte mismatches fail admission.

An artifact-set record never lists itself. It may list an index or ledger that
does not refer back to the set. A final outer release set may list the
completion seal, but the completion seal cannot refer to that outer set.

## Acyclic lifecycle

```text
manifest + public inputs + common kit index
                    |
                  freeze
                 /      \
       prediction A    prediction B
                 \      /
                oracle reveal
                      |
                  completion
```

Hashes prove dependency, not time. Commit ancestry and annotated remote tags
establish that both predictions existed before oracle reveal. Timestamps are
not evidence, and no record names the Git commit that contains itself.

### Freeze

The freeze seal binds:

- exact candidate language, core-semantics, and error-catalog source IDs and
  raw bytes;
- the complete generated manifest raw digest;
- the common oracle-free public-input artifact set;
- the selected runtime-independence review bytes; and
- Model A and Model B kit artifact sets plus their one common kit index.

The two kit indexes must have equal semantic and raw identities. The freeze
seal does not contain an oracle artifact-set reference: the manifest raw hash
already commits withheld oracle hashes, while the kits omit them. The reveal
set is created only after both prediction seals.

### Prediction

Each prediction seal binds one producer, the common freeze and kit index, the
producer's isolated source and executable artifact sets, the exact executable
path and bytes, the candidate implementation identity over that executable,
the actual runtime-evidence record, the complete prediction ledger, and its
transcript artifact set.

Runtime evidence is generated by the arbiter from the launcher used for every
recorded run, not supplied by the model. Process policy
`selta.evidence.conformance-process/s2-1` means a newly created empty working
directory, an inherited environment cleared and replaced by exactly the
record's environment rows, and no command-line arguments other than the
admitted template. The fixed `__SELTA_S2_PRIVATE_HOME__` and
`__SELTA_S2_PRIVATE_TMPDIR__` environment values are structurally replaced by
distinct, newly created empty directories beneath the private run root for
each process; no other environment value is rewritten. For `direct`, the
sealed executable is materialized mode `0555` and is the launcher, the argument
template is empty, and there are no probes. For `retained_runtime`,
`launcher_path` is normalized and relative to the referenced runtime
artifact-set root, with no `.` or `..` segment. The launcher is taken from that
retained set, and its bytes must match `launcher_bytes_sha256` before every
probe and model run.

The retained runtime artifact set does not contain its own record or the
runtime-evidence record. Under profile
`selta.evidence.conformance-runtime/cpython-3.9-stdlib-1`, it contains exactly
the regular files at `Python3`, `bin/python3.9`, beneath `Resources/`, and
beneath `lib/python3.9/`, excluding every `site-packages` or `__pycache__`
subtree and every `.pyc` file. No symbolic link is admitted. These files are
copied from the selected Xcode CPython prefix before kit release, retained with
the Model-B kit, materialized with the same relative layout, and checked
against the set immediately before every process. Directories are materialized
mode `0555`; `Python3`, `bin/python3.9`,
`Resources/Python.app/Contents/MacOS/Python`, and regular
`lib/python3.9/lib-dynload/*.so` files are mode `0555`; every other regular file
is mode `0444`. File modes are deterministic process policy, not content
identity. Its framework engine, standard library, and `lib-dynload` modules are
therefore prediction inputs; the host operating-system ABI remains the
explicitly limited platform precondition in the runtime review.

In a retained-runtime argument template,
`__SELTA_S2_EXECUTABLE__` occurs exactly once and is structurally replaced by
the sealed executable path.

Both candidate-1 records have exactly the environment rows `HOME` set to
`__SELTA_S2_PRIVATE_HOME__`, `LANG=C`, `LC_ALL=C`, and `TMPDIR` set to
`__SELTA_S2_PRIVATE_TMPDIR__`. Candidate-1 Model B uses retained launcher
`bin/python3.9`, arguments
`["-I", "-S", "-B", "__SELTA_S2_EXECUTABLE__"]`, and exactly one
`python-runtime` probe against that launcher. Its arguments are exactly `-I`,
`-S`, `-B`, `-c`, and this single program string:

```text
import os,sys;p=os.path.realpath(sys.base_prefix);print(sys.implementation.name);print('.'.join(map(str,sys.version_info[:3])));print(sys.flags.isolated);print(sys.flags.no_site);print(sys.flags.dont_write_bytecode);print(os.path.relpath(os.path.realpath(sys.executable),p));[print(os.path.relpath(x,p)) for x in sys.path]
```

It must exit zero with empty stderr and exact UTF-8 stdout consisting of the
newline-terminated lines `cpython`, `3.9.6`, `1`, `1`, `1`, `bin/python3.9`,
`lib/python39.zip`, `lib/python3.9`, and `lib/python3.9/lib-dynload`, in that
order. Model A uses direct invocation. The
runtime-evidence record is retained in the producer's transcript set as well
as referenced by the prediction seal; its producer must equal the seal
producer. The referenced runtime set must be the same set already enclosed by
the frozen Model-B kit. The authored manifest input names its profile, retained
root, and generated artifact-set record path but contains no runtime hash.

The ledger has exactly one zero-exit run for every `admit_environment` and
`assess` case in the kit index. Requests are exact UTF-8 JCS. Complete stdout
bytes contain one schema-valid response and no second value; diagnostics are
retained only as stderr. Every returned outcome execution carries the request
implementation digest. The transcript set contains only the runtime-evidence
record, the ledger, and one request, response, and stderr artifact per run.

Model A and Model B source, executable, runtime, and transcript material never
crosses sets. Source, executable, runtime evidence, ledger, and transcripts are
sealed before any oracle artifact is revealed.

The two prediction seals must carry unequal `executable.bytes_sha256` values
and unequal `executable.implementation` values. Equality in either field
invalidates the pair before oracle comparison; two producer labels cannot turn
one executable into independent evidence. This is a cross-record admission
rule and therefore is not expressible in either local prediction-seal variant.

### Completion

The completion seal binds the protected pre-S2 baseline, the common freeze,
both distinct prediction seals, the exact oracle reveal closure, the arbiter
source and executable sets, the minimal completion ledger and detailed
comparison artifacts, the review set, and final verification evidence.

The completion ledger lists every required `(case, subject)` evaluation once:
Model A and Model B for admission, assessment, and per-model comparisons; the
arbiter for finite and cross-model comparisons. `pass` means that every
applicable schema, identity, closure, resource, oracle, trap, projection,
finite-result, and comparison check actually ran. Global manifest closure,
law/named/error coverage, kit exclusion, and prediction completeness are
ledger admission preconditions rather than duplicated status rows.

Before evaluating any cross-model case, the arbiter loads both referenced
prediction seals and enforces the unequal executable-byte and implementation-
identity rule above. Completion cannot use package inequality as a circular
proof of that precondition.

The review set contains exactly runtime independence, arithmetic bounds, error
reachability, formal consistency, privacy boundary, identity integrity, and
compatibility boundary. The verification set contains stable workspace and
complete corpus/admission evidence. The runtime-independence review in that set
must have the same raw digest as `runtime_review_bytes_sha256` in the referenced
freeze; replacement or omission invalidates completion. No referenced set
contains the completion seal.

## Implementer-kit boundary

The kits' common producer-neutral closure contains only:

- documents 10 through 23 and explicitly public conformance guidance;
- case, request, and response schemas;
- four environments and their exact resolver closure;
- `admit_environment` and `assess` case records and package sources; and
- public canonicalization vectors.

They omit the generated manifest, every oracle and expected-only dependency,
finite expected values, semantic call ledgers, intermediate values, forbidden
mutations, answer-bearing fixture READMEs, corpus generators, reviews, arbiter
source, repository history, build caches, and all other-model material.

Model A alone receives the minimal stable `selta-core` and `selta-protocol`
source boundary plus its pinned generated Cargo workspace. Model B receives no
Rust source or Cargo material; it receives the frozen retained-runtime set
selected above. The kit builder starts only from the declared public closure
and never traverses a case's oracle edge; a hard deny policy cannot be
overridden by authored input.

## Arbiter discipline

The isolated arbiter may admit Selta schemas, verify values, recompute wire
identities, validate already returned packages, derive the exact document-16
`ConformanceProjection`, evaluate the generic finite DSL, compare observations,
build kits, run sealed executables, and validate the record DAG.

It must never parse package input under the candidate error schedule, validate
evidence relations, execute semantic descriptors, construct state, closures,
conclusions, resources, errors, or expected packages, export candidate helpers,
or become a dependency of a model or future reference crate. Its narrow
post-hoc identity checker remains private and independently reviewed. Crossing
that boundary would create a third assessment implementation and invalidates
the run.

## Invalidation and non-claims

Changing documents 15, 16, or 20 changes a candidate source identity and
requires a new environment revision and full run. Changing a conformance
schema, case, oracle, manifest, kit rule, or private protocol after freeze
invalidates that freeze and every dependent record. A new run never edits old
seals into apparent continuity.

Completion establishes only S2 conformance for the selected artifacts and
real runtimes. It does not establish empirical calibration, unrestricted
semantic correctness, confidentiality, Windows support, L4 legacy byte
equivalence, S3 reference-runtime readiness, or product value.
