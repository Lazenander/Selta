# 26 — Windows product-runtime compatibility

Windows product-runtime compatibility is narrower than formal operating-system
support for the candidate evidence calculus. The latter still requires the same S2
conformance corpus and secure arbiter on Windows under
[16-evidence-semantics.md](16-evidence-semantics.md). This document fixes the first
runtime profile and its evidence gate without weakening that rule.

## Profile W1

The W1 candidate targets native `x86_64-pc-windows-msvc` on GitHub's rolling
`windows-2022` image family. Each run records the concrete image version; Rust, Node,
and action implementations are pinned. W1 contains:

- `selta-core` and `selta-protocol`;
- `seltad` over a TCP listener;
- the `selta` HTTP CLI;
- the bundled SQLite catalog on a local filesystem;
- builtin verifiers, including allowlisted commands;
- server-spawned Node hosts over newline-delimited JSON-RPC on stdio; and
- application-provided Node hosts over WebSocket.

The default configuration is inside this profile: it listens on
`127.0.0.1:7466` and uses SQLite. A `unix:` listener is rejected before hosts or
storage are opened. Named pipes are not silently substituted for Unix sockets.

Command timeout kills and reaps its immediate child, and RPC shutdown waits,
escalates, and reaps its immediate child. W1 does not promise
descendant-process-tree containment.
A verifier that starts descendants must stop them itself until a Windows Job Object
adapter and its side-effect tests are present.

## Deliberate exclusions

The following are not part of W1:

- `storage = "files"`; its physical-name, locking, and crash-replacement contract
  needs a separate Windows filesystem design;
- Unix-domain sockets, Windows named pipes, Windows services, and service-control
  shutdown;
- Windows 10 or 11 client behavior, standard-user ACL/UAC behavior, ARM64, SMB,
  removable drives, and network filesystems;
- the isolated S2 conformance arbiter, whose secure no-follow artifact traversal has
  no Win32 handle/reparse-point adapter; and
- the excluded real-Codex experiment runner, whose real-run process and credential
  containment is Unix-specific.

Each excluded daemon configuration fails closed; isolated research tools remain
outside the product workspace. The public file-catalog type remains visible for API
stability but its constructor rejects Windows; migration is non-Windows-only.
Unvalidated platforms are simply unclaimed. No exclusion may be converted into a
silent fallback.

## Evidence gate

Source inspection and cross-compilation are preparation, not Windows runtime
validation. W1 becomes **runtime-validated** only when a native Windows job passes all
of these gates from a fresh checkout:

1. format and Clippy (`-D warnings`) for the complete product workspace;
2. build of `seltad.exe` and `selta.exe` with the MSVC toolchain;
3. every workspace Rust test, including TCP HTTP, SQLite, stdio RPC, WebSocket, and
   command timeout tests with real processes;
4. the reference TypeScript SDK tests under the pinned Node version; and
5. exact-byte checkout checks for committed corpora and ledgers whose line endings
   are part of their identity.

The workflow is the executable runtime-evidence definition. A failing or skipped
required step means the runtime profile is not validated. Interactive Ctrl+C and
graceful host drain remain manual/unverified on Windows because CI force-termination
is not a console-control event. Real Lean and provider-backed judge tests remain
separate tool/release gates because their external installations and secrets are not
product portability evidence.

## Local virtual machines

A local Windows VM is not a prerequisite for W1. The native GitHub runner is the
first correctness gate and covers the x64/MSVC target that a Windows-on-Apple-Silicon
VM cannot replace. A local Windows 11 ARM VM becomes useful when interactive debugging,
ARM64, client-only behavior, ACL/UAC, or service integration enters scope.
