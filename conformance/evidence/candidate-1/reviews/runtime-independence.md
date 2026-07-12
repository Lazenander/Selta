# S2 conformance runtime independence review

> **Status: runtime pair selected; model source remains blocked until the
> input-only kits and prediction protocol are sealed.** This record selects
> disposable conformance tools, not a product runtime or supported platform.

## Selected pair

| Model | Runtime | Permitted shared boundary | Sealed artifact |
|---|---|---|---|
| A | Rust 1.93.0 (`aarch64-apple-darwin`, LLVM 21.1.8) | Stable Selta schema-source admission and value verification only | One release executable file |
| B | Host `/usr/bin/python3` 3.9.6, standard library only | Public schemas and protocol bytes; no Model-A or candidate helper code | One executable zip application |

Rust remains the default implementation language for repository-owned non-UI
work. The second model deliberately uses a different parser, object model,
integer implementation, control flow, runtime, and packaging format because
heterogeneous failure modes are the evidence sought by S2. It is disposable
black-box test evidence, not a Python Selta implementation or future library.

## Independence constraints

- Separate implementers receive separate input-only kit directories and no
  surrounding repository history.
- Neither kit contains expected packages, oracle records, the corpus authoring
  tools, generated candidate types, the arbiter, or the other model.
- The two models share no candidate parser, identity, relation, semantic,
  closure, accounting, error, or output-construction code.
- Model A may call stable `selta-core` only at the public schema admission and
  strict value-verification boundary allowed by document 17. Model B implements
  the required public shape checks independently from the supplied schemas.
- Each implementer seals its source digest and prediction ledger before any
  oracle is revealed. Build and prediction logs are retained separately.
- The arbiter contains no assessment semantics. It verifies returned artifacts
  from public schemas and identities, constructs document 16's exact
  `ConformanceProjection`, and evaluates manifest-declared comparisons.

## Artifact identity and invocation

The harness hashes the exact executable file supplied to it under document
16's implementation domain and inserts that digest into each request. Model A
is the compiled release binary. Model B is the exact zip-application file;
the fixed Python runtime version is an external conformance precondition, not
part of the file preimage. Both consume one request on standard input and emit
one response on standard output under the private schemas in this directory.

No source file, directory walk, interpreter installation, dependency cache,
native path, process ID, or build timestamp is an implementation-identity
preimage. Diagnostics use standard error only.

## Current platform claim

This selection permits one real S2 run on the present Apple-silicon system. It
does not claim macOS product support and says nothing about Windows behavior.
The candidate judgments remain platform-neutral; another operating system must
run the same public corpus before acquiring its own support evidence.

Model source work may begin only after the complete manifest, context closure,
held-out split, and input-only kit digests are frozen.
