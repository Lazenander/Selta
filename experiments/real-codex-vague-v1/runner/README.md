# Real-Codex experiment runner

This standalone Rust tool executes the frozen evidence-assessor experiment. It
is excluded from Selta's product workspace: experiment dependencies and model
orchestration do not enter the core library.

Each `run` creates a fresh output directory and freezes `manifest.json` and
`jobs.jsonl` before any model call. Inputs, prompts, and both exact canonical
schemas are copied under `artifacts/`; calls and later scoring use those
snapshots. Jobs are the seeded ordering of the complete
prompt-case cross-product. Every job is explicitly a `scored_first_attempt`;
the runner never retries or replaces it. Up to four jobs may run concurrently,
each in a fresh empty working directory.

Multiple `--prompt ID=PATH` arguments are accepted so the held-out baseline and
selected prompt can be interleaved in one run. The model and reasoning setting
are required rather than silently defaulted.

```sh
cargo run --manifest-path runner/Cargo.toml -- run \
  --mode development \
  --inputs corpus/dev.inputs.jsonl \
  --prompt p0=prompts/p0.txt \
  --schema schemas/response.schema.json \
  --selta-schema schemas/response.selta.json \
  --output runs/dev-p0 \
  --model MODEL_ID \
  --reasoning low \
  --seed 1643361912 \
  --concurrency 4
```

Add `--dry-run` to validate and freeze only the manifest and job plan. This
performs no model call. `run` cannot receive an oracle: scoring is a separate
`validate --run ... --oracle` action after predictions have been frozen.

Modes enforce only call shape, never corpus validity: `development` is 24 cases
and one or two prompts; `heldout` is 24 cases and one or two prompts including
`p0`; `engineering-smoke` is one prompt and at most four cases.

The JSON Schema is only the Codex structured-output guard and intentionally
omits unsupported `uniqueItems`. The second schema is strictly admitted as Selta
1 against `AdmissionPolicy::pure_only()`. Every raw response must pass that
canonical Selta contract before deterministic within-array uniqueness,
cross-side disjointness, and exact-quotation checks run. Removing a transport
keyword therefore does not weaken the admitted scoring value.

## Assessor isolation

Each child has an empty working directory and a fresh `CODEX_HOME` that starts
with only a private writable copy of `auth.json`. Inherited `CODEX_*`, `OPENAI_*`,
and `CHATGPT_*` variables are removed. The child and descendants run in a
dedicated process group so timeout termination cannot orphan a model call. This
engineering revision is qualified against Codex CLI `0.144.1`.

One runner constant freezes this exact optional-capability denylist, emitted as
one `--disable` pair per entry and copied into the manifest:

```text
plugins
apps
shell_tool
image_generation
goals
hooks
personality
multi_agent
shell_snapshot
```

The command also carries one ordered, manifest-recorded neutral-context config
list. It freezes `web_search="disabled"`, `approval_policy="never"`, all three
skill-instruction paths, environment, permission, and collaboration instruction
injection, the experimental request-input tool, and legacy notifications. It
also empties Terra/Sol's two model-selected multi-agent hint strings and limits
the session to its one root slot. Its last entry sets `instructions="Follow the
user instruction exactly."`; the manifest also records that exact neutral
instruction and its digest. This harness constant is never optimized; `p0`
remains the only task prompt under optimization. The exact config list is visible
in every command template and covered byte-for-byte by the runner tests.

The denylist is the source-traced minimum of direct optional gates for CLI
`0.144.1`, not an inheritance model or a promise that the CLI supplies no core
or model-selected tool schemas. `multi_agent` records intent but is overridden
by Terra/Sol metadata; the one-slot pin is operational containment, not schema
removal. The broader config list remains necessary because Codex's ambient
channels are independent, and shell snapshots can start a user shell when model
shell tools are off. The raw-event parser is therefore authoritative: any
observed tool event makes the attempt operationally invalid. After the child
exits, the runner also rejects an isolated home containing a `plugins`,
`remote_plugin_catalog`, or `shell_snapshots` path component.

## Non-evidentiary smoke record

`runs/smoke-terra-low-001` is retained unchanged as a failed infrastructure
record. Codex CLI `0.142.5` was too old for `gpt-5.6-terra`, and the attempt
revealed remote plugin synchronization into a nominally fresh `CODEX_HOME`.
There were no development-corpus calls, so the bundle is neither semantic
evidence nor a prompt result. The fix belongs to the runner and the `0.144.1`
execution contract; the historical bundle must not be rewritten.

`runs/smoke-terra-low-002` is likewise retained unchanged. CLI `0.144.1` and the
isolated configuration succeeded, but the API rejected `uniqueItems` in the
transport output schema before model judgment. It made no development-corpus
calls and provides no semantic or prompt-quality evidence. The compatibility
revision removes that keyword only from the live transport schema and its exact
canonical checker; deterministic post-Selta uniqueness and disjointness remain.
Bound validation recognizes the old schema only in `engineering-smoke` mode and
by its single frozen SHA-256, so both historical smoke bundles remain
verifiable; a new `run` cannot use it.

A completed run retains:

- `manifest.json`: exact artifacts, model configuration, command template, and
  ordering policy;
- `jobs.jsonl`: the pre-call scored-attempt order;
- `raw/*.events.jsonl`, `*.stderr.txt`, and `*.response.json`: unmodified Codex
  artifacts;
- `raw-index.json`: paths, byte lengths, and digests for every raw artifact;
- `predictions.jsonl`: mechanically checked outcomes in job order;
- `completion.json`: digests of frozen result artifacts.

Bound `validate` reconstructs jobs, verifies manifest/completion/snapshot/raw
digests, reparses successful raw responses and event usage, and then scores the
exact supplied oracle bytes. Loose artifact validation remains diagnostic only
and cannot receive an oracle.

```sh
cargo run --manifest-path runner/Cargo.toml -- validate \
  --run runs/dev-p0 \
  --oracle corpus/dev.oracle.jsonl \
  --output runs/dev-p0/metrics.json
```

Primary state metrics require every predicted quotation to contain a distinct
same-polarity minimal oracle span. Raw presence-state metrics remain separate.
This deterministic rule is conservative when an oracle omits valid alternative
evidence; it is engineering evidence, not general semantic proof.

After a held-out oracle and nonce are disclosed, verify their precommitted bytes
without printing labels:

```sh
cargo run --manifest-path runner/Cargo.toml -- verify-commitment \
  --commitment corpus/holdout.commitment.json \
  --inputs /withheld/holdout.inputs.jsonl \
  --oracle /withheld/oracle.jsonl \
  --nonce /withheld/nonce.txt
```

For a heldout run, supply the external input and `--commitment`; the runner
verifies their digest/count agreement before calls and snapshots both. Later
`validate --run --oracle ... --nonce ...` reuses the bound commitment and refuses
to score without the nonce. Before disclosure, commit
and push the complete portable run directory to the private remote. That Git
object is the external receipt for the completion digest.

The author-constructed corpora remain engineering fixtures. Passing this runner
does not turn them into the three-human-adjudicated pilot required by the parent
protocol.
