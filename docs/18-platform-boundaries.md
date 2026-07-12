# 18 — Platform boundaries

> **Status: portability design and gap record.** This document does not claim
> Windows compatibility and does not authorize speculative Windows code. A
> Windows implementation and support claim require compilation and conformance
> tests on a real Windows environment.

## Principle

Selta has a logical subset, but the current crate boundary is not yet the
logical boundary. `selta-core` currently contains schema, admission, folds, and
report values alongside native command execution, temporary artifacts, Tokio,
and the stdio `RpcHost`. The crate as shipped is therefore not a pure or wholly
platform-neutral crate.

The intended separation is:

```text
pure logical subset
    schema + strict admission + structural checking + folds + report projection
    future evidence admission + interpretation + conclusion projection

normalized observation boundary
    extension envelope/error + timeout/cancellation + cache/resource outcome

execution and platform shell
    current effectful selta-core pieces + storage + listener + shutdown
    process + temp artifact + clock + client and host transport
```

Platform independence is conditional on observations, not a claim that native
environments behave identically:

> Given equal admitted logical inputs, semantic revisions, abstract resource
> limits, and equivalent ordered normalized observations, the logical subset
> produces the same identity, error classification, ordering, report, and
> evidence result.

A filesystem failure, scheduler delay, process exit, or unavailable transport
may produce a different normalized observation on another platform. That
difference is explicit input at the observation boundary; it is not permission
for an adapter to reinterpret an equivalent observation or silently weaken its
atomicity, durability, cancellation, or resource promise.

## Support vocabulary

Selta uses these terms precisely:

| Term | Meaning |
|---|---|
| Platform-neutral | The contract contains no native platform concept |
| Adapter exists | An implementation has been written for a platform |
| Builds | The complete selected feature set compiles there |
| Validated | The platform conformance suite passes on a real machine |
| Supported | Builds, validation, documentation, and maintenance policy are all present |

Designing an adapter boundary does not satisfy the next three levels. Candidate
evidence work claims only that its core contract is platform-neutral.
Candidate schema admission additionally caps every Selta `Node.len` bound at
`u32::MAX` before stable Selta converts it to `usize`; a schema accepted by the
candidate therefore cannot depend on whether the host has 32- or 64-bit
pointers.

## Logical subset and future split

These areas are logical or can be extracted behind normalized observation
interfaces:

- JSON value and schema models;
- strict raw-source admission and canonical JSON pointers;
- verification folds and report types;
- registry declarations and logical settings resolution;
- content identities and evidence graphs;
- causal ordinals and abstract resource counters; and
- pure interpretation and conclusion projection.

They MUST NOT accept native paths, file descriptors, process IDs, signals,
socket addresses, executable suffixes, or OS error numbers as semantic inputs.
An application may carry such data inside an explicitly typed payload, but it
does not change core behavior.

This list describes a semantic boundary, not the current `selta-core` package
boundary. A future pure evidence crate starts on the logical side. Existing
verification code should be split only when a real adapter or reference
implementation needs the seam; until then `selta-core` remains a mixed
composition crate and must not be cited as evidence that the split already
exists.

## Current coupling and unsupported assumptions

### Listener and shutdown

The daemon currently references `tokio::net::UnixListener` in
[`crates/seltad/src/main.rs`](../crates/seltad/src/main.rs) without a Unix
compilation gate. Only SIGTERM handling is gated. The overloaded `listen`
string distinguishes a `unix:` prefix from TCP but provides no endpoint type,
named-pipe adapter, access-control contract, or Windows service shutdown source.

Unix-socket startup removes the configured path without first establishing the
expected file type or a cleanup/ownership policy. These are daemon-shell
concerns, not reasons to condition the core.

### CLI and Unix-socket transport

The daemon can listen only on a Unix-domain socket, but
[`selta-cli`](../crates/selta-cli/src/main.rs) treats `--server` as an HTTP URL
and sends every request through `ureq`. It has no Unix-domain-socket transport
or endpoint negotiation. Consequently, the bundled CLI cannot connect to a
daemon configured with only `listen = "unix:..."`; the server-side Unix option
and the client transport surface are currently asymmetric.

A future `ClientTransport` must make supported endpoint combinations explicit.
The existence of a Unix listener is not evidence that the complete CLI/server
workflow supports that listener.

### File catalog

[`FileStorage`](../crates/seltad/src/storage/files.rs) relies on filesystem
behavior not expressed by the `Storage` trait:

- schema-version allocation is read-max-write without an interprocess lock;
- pool metadata is updated in place;
- statistics use one fixed temporary filename and rename it over the target;
- crash durability does not include directory and file synchronization;
- case-distinct logical names may alias on a case-insensitive filesystem;
- reserved names and platform path-length limits are not represented;
- ambient permissions and ACLs define access; and
- externally introduced symlinks are not governed by a catalog policy.

Rename-over-existing and durability behavior differ by platform and
filesystem. The current backend must not be described as uniformly atomic or
portable until its trait states, and each adapter proves, the required
semantics.

SQLite is a promising cross-platform backend but does not itself prove support.
Locking, WAL, durability, removable media, and network filesystem behavior must
be validated on the deployed filesystem.

### Synchronous storage inside asynchronous handlers

The current [`Storage`](../crates/seltad/src/storage/mod.rs) trait is
synchronous. `FileStorage` performs synchronous filesystem work and
`SqliteStorage` performs synchronous `rusqlite` work behind a mutex. Async Axum
handlers in [`api.rs`](../crates/seltad/src/api.rs) call both implementations
directly, and the periodic stats task also calls synchronous storage directly;
the application does not consistently move those calls to a dedicated blocking
executor.

Slow filesystems, SQLite busy waits, large imports, or lock contention can
therefore block a Tokio worker. File and SQLite backends share this gap even
though their native blocking mechanisms differ. A future storage boundary must
either be honestly asynchronous or prescribe a dedicated blocking execution
and concurrency policy. Merely changing the backend does not fix the async
contract.

### Processes and temporary artifacts

The command builtin and RPC host use native subprocesses directly.

- `kill_on_drop` concerns one child handle, not a process-tree containment
  contract;
- the existing post-timeout side-effect test is Unix-only;
- executable discovery, extensions, environment inheritance, quoting, and exit
  status differ by platform;
- temporary paths are converted through lossy strings; and
- a temporary file may remain open while another process receives its path.

These concerns belong behind process and temporary-artifact adapters. Evidence
semantics receives normalized call, result, timeout, cancellation, and resource
facts from an admitted adapter or acquisition recorder; it neither derives
those facts from native state nor supervises the process.

### RPC host shutdown

[`RpcHost::shutdown`](../crates/selta-core/src/host/rpc.rs) sends a `shutdown`
request and waits up to two seconds for the RPC response. It does not then wait
for and reap the child, enforce a separate process-exit grace period, or
escalate to a kill when a host acknowledges shutdown but remains alive. The
child uses `kill_on_drop`, which applies when the final host handle is dropped
and covers the immediate child rather than a declared descendant-containment
policy.

Graceful server shutdown also has no explicit host-drain sequence proving that
all registered hosts were asked to stop, given a grace interval, killed if
necessary, and reaped. A future `ProcessSupervisor` must make each transition
and its timeout observable.

### TypeScript host cancellation

The reference [`@selta/extension` SDK](../packages/extension/index.js) accepts a
JSON-RPC `cancel` notification but deliberately lets the in-flight handler
finish. Its public handler type receives no cancellation token or
`AbortSignal`. Dropping the server-side waiter therefore does not stop provider
calls, subprocesses, writes, or other effects already running inside a
TypeScript verifier.

Protocol cancellation is currently best effort, not an effect-containment
guarantee. A future SDK contract must pass a cancellation capability to the
handler, define acknowledgement and late-result behavior, and test that a
cooperative handler stops observable effects. Uncooperative handlers remain a
process-supervision concern.

## Required adapter contracts

Future portability work should introduce only boundaries demanded by a real
implementation.

### Clock

Provides monotonic deadline accounting and, separately, optional wall time.
Wall time never establishes causal order. Tests inject a logical clock.

### ListenerEndpoint and ServerListener

The configuration DSL uses closed logical endpoint variants. Platform adapters
may implement TCP, Unix-domain socket, named pipe, or another transport. Each
variant declares address validation, access-control, stale-endpoint cleanup,
and shutdown behavior.

### ShutdownSource

Normalizes console interruption, service control, signal, and programmatic
shutdown into one graceful-shutdown event without exposing native signal
numbers to server logic.

### ClientTransport

Separates HTTP request semantics from TCP, local socket, named pipe, proxy, and
TLS mechanics. The CLI depends on this interface rather than on a platform
listener type. Each server endpoint advertised as usable by the CLI has a
real-environment interoperability test.

### ProcessSupervisor

Owns executable resolution, arguments, environment, working directory,
standard streams, cancellation, deadline, exit normalization, and process-tree
containment. Shutdown is a state machine: request graceful stop, wait a declared
grace interval, escalate, wait and reap, then emit the normalized outcome. A
platform implementation states exactly what descendants it can terminate and
proves that promise with real side-effect tests.

### NormalizedObservationRecorder

Maps native adapter events into the closed logical observation vocabulary. The
recorder supplies the normalized observation to verification or evidence code
and binds the mapping revision, causal identity, source adapter, and retained
native detail or typed commitment. The logical subset does not inspect an OS
error number, infer a timeout from wall time, or invent a cancellation event.

Two platform runs are comparable only after their recorder outputs are shown to
be equivalent under the same normalization revision. Recorder admission proves
which mapping ran; it does not prove that an opaque native source told the
truth.

### TempArtifact

Provides a byte artifact with declared lifetime, sharing, close-before-exec,
path encoding, cleanup, and permission semantics. Core code does not convert a
native path to a lossy string.

### DurableFileOps and CatalogLock

Define create-if-absent, atomic replace, durable commit, directory sync,
locking, and crash recovery as logical operations. Adapters either meet the
declared guarantee or reject that backend profile.

### LogicalCatalogName

Separates a portable logical identifier from its physical path mapping. The
mapping handles case folding, reserved names, length limits, normalization, and
collision detection without changing the logical identifier.

### Storage

The existing storage interface should eventually state transaction isolation,
version-allocation atomicity, update atomicity, durability, locking scope, and
failure recovery. It must also state whether calls are async or require a
blocking executor and the maximum concurrency allowed there. File and SQLite
implementations share those semantics rather than merely sharing method names.

## Dependency direction

The intended future dependency graph is one-way:

```text
pure logical crates or extracted logical modules
    schema/admission/folds/report + evidence semantics
                         ^
                         |
       logical traits + normalized observations
                         ^
                         |
current effectful selta-core facade + seltad + CLI + SDK adapters
```

The future pure layer does not depend on daemon configuration, Tokio listeners,
native processes, filesystem catalogs, or a particular async runtime. Platform
shells may depend on it. This is a target split: current `selta-core` contains
code on both sides and must not be placed wholesale in the top box.

An adapter may be selected by configuration or build feature. Conditional
compilation belongs at adapter construction and implementation boundaries, not
inside schema or evidence semantics.

## Future platform conformance

Each claimed platform runs, in a real environment:

- the complete pure workspace suite;
- listener startup, access control, stale-endpoint, shutdown, cleanup, and CLI
  interoperability for every advertised endpoint;
- catalog concurrent version allocation, case collision, reserved name,
  atomic update, crash interruption, durability, and async-runtime
  responsiveness while File and SQLite operations block or contend;
- process success, malformed exit, timeout, cancellation, descendant
  containment, graceful-shutdown timeout, kill escalation, reaping,
  environment, and non-ASCII path cases;
- TypeScript SDK cancellation with a cooperative handler and observable
  side-effect check;
- temporary-artifact sharing and cleanup;
- SQLite locking and recovery on supported filesystems;
- equivalence vectors showing that different adapters produce the same logical
  result when their normalized observations are equivalent; and
- all evidence canonicalization and conformance vectors.

Tests from another operating system, mocks, cross-compilation, and successful
compilation are useful preparation but are not validation evidence.

## Current decision

This research track will:

- keep every new evidence document and pure semantic operation platform-neutral;
- treat the logical subset of current verification separately from the mixed
  `selta-core` crate boundary and extract it only when an implemented seam needs
  that split;
- record current coupling and the required seams;
- avoid adding new direct OS dependencies to pure code; and
- leave concrete Windows adapters and compatibility claims to work performed
  and tested on a Windows computer.

It will not refactor unrelated daemon code merely to create unused abstraction
layers. Platform seams should be introduced when the corresponding behavior is
implemented and can be tested.
