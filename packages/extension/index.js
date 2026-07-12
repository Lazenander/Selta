// @selta/extension — the reference host SDK (docs/05).
// Handles ndjson JSON-RPC framing, the manifest, and concurrency; authors
// write only the verifier function: (value, ctx) → { verdict, delta? }.

import readline from "node:readline";

const registry = new Map();
const assessors = new Map();

function normalizeDelta(delta) {
  return typeof delta === "string" ? { message: delta } : delta;
}

function normalizeRefutation(delta) {
  const normalized = normalizeDelta(delta);
  if (
    normalized === null
    || typeof normalized !== "object"
    || Array.isArray(normalized)
    || typeof normalized.message !== "string"
    || normalized.message.trim() === ""
  ) {
    throw new TypeError("a delta must be a non-empty message string or an object with one");
  }
  return normalized;
}

function context(params) {
  return {
    config: params.config,
    settings: params.settings ?? {},
    path: params.path,
    root: params.root,
    env: params.env,
    budget: params.budget,
  };
}

export function pass() {
  return { verdict: "pass" };
}

export function fail(delta) {
  return { verdict: "fail", delta: normalizeDelta(delta) };
}

export function support() {
  return { support: true };
}

export function refute(delta) {
  return { support: false, refute: normalizeRefutation(delta) };
}

export function both(delta) {
  return { support: true, refute: normalizeRefutation(delta) };
}

export function neither() {
  return { support: false };
}

export const host = {
  verifier(name, options, handler) {
    const normalized = options ?? {};
    if (
      normalized.semanticRevision !== undefined &&
      (typeof normalized.semanticRevision !== "string" || normalized.semanticRevision.trim() === "")
    ) {
      throw new TypeError(`verifier '${name}': semanticRevision must be a non-empty string`);
    }
    if (normalized.cacheable === true && normalized.semanticRevision === undefined) {
      throw new TypeError(`verifier '${name}': cacheable verifiers require semanticRevision`);
    }
    if (
      normalized.cacheable === true &&
      (normalized.determinism ?? "nondeterministic") !== "deterministic"
    ) {
      throw new TypeError(`verifier '${name}': only deterministic verifiers may be cacheable`);
    }
    if (normalized.cacheable === true && normalized.effectClass !== "pure") {
      throw new TypeError(`verifier '${name}': cacheable verifiers must declare effectClass 'pure'`);
    }
    if (normalized.needs !== undefined) {
      if (!Array.isArray(normalized.needs)) {
        throw new TypeError(`verifier '${name}': needs must be an array`);
      }
      const seenNeeds = new Set();
      for (const need of normalized.needs) {
        if (need !== "root" && need !== "env") {
          throw new TypeError(`verifier '${name}': unknown need '${String(need)}'`);
        }
        if (seenNeeds.has(need)) {
          throw new TypeError(`verifier '${name}': duplicate need '${need}'`);
        }
        seenNeeds.add(need);
      }
    }
    registry.set(name, { options: normalized, handler });
  },

  assessor(name, handler) {
    if (!registry.has(name)) {
      throw new TypeError(`assessor '${name}': register its verifier first`);
    }
    assessors.set(name, { handler });
  },

  run(info = { name: "selta-ts-host", version: "0.2.0" }) {
    const rl = readline.createInterface({ input: process.stdin, terminal: false });
    const write = (message) => process.stdout.write(JSON.stringify(message) + "\n");

    rl.on("line", async (line) => {
      let message;
      try {
        message = JSON.parse(line);
      } catch {
        return; // not ours; ignore
      }
      const { id, method, params } = message;
      const reply = (result) =>
        id !== undefined && write({ jsonrpc: "2.0", id, result });
      const replyError = (code, text) =>
        id !== undefined && write({ jsonrpc: "2.0", id, error: { code, message: text } });

      if (method === "initialize") {
        reply({
          host: info,
          extensions: [...registry.entries()].map(([name, { options }]) => ({
            name,
            determinism: options.determinism ?? "nondeterministic",
            semantic_revision: options.semanticRevision ?? null,
            cacheable: options.cacheable ?? false,
            effect_class: options.effectClass ?? null,
            accepted_input: options.acceptedInput ?? null,
            config_schema: options.configSchema ?? null,
            needs: options.needs ?? [],
            settings_schema: options.settingsSchema ?? null,
            delta_schema: options.deltaSchema ?? null,
          })),
        });
      } else if (method === "verify" || method === "assess") {
        const entries = method === "verify" ? registry : assessors;
        const entry = entries.get(params?.ext);
        if (!entry) {
          const text = method === "verify"
            ? `unknown extension '${params?.ext}'`
            : `extension '${params?.ext}' has no assessor`;
          return replyError(-32601, text);
        }
        try {
          reply(await entry.handler(params.value, context(params)));
        } catch (error) {
          replyError(-32000, String(error?.message ?? error));
        }
      } else if (method === "shutdown") {
        reply({});
        process.exit(0);
      } else if (method === "cancel") {
        // Best-effort notification: the reference SDK lets in-flight handlers
        // finish because the caller has already stopped waiting.
      } else {
        replyError(-32601, `unknown method '${String(method)}'`);
      }
    });
  },
};
