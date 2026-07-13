import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { both, fail, neither, refute, support } from "../index.js";

test("legacy fail helper retains its exact normalization behavior", () => {
  assert.deepEqual(fail("no"), { verdict: "fail", delta: { message: "no" } });
  assert.deepEqual(fail(""), { verdict: "fail", delta: { message: "" } });
  assert.deepEqual(fail(undefined), { verdict: "fail", delta: undefined });
});

test("assessment helpers expose the closed four-state wire shape", () => {
  assert.deepEqual(support(), { support: true });
  assert.deepEqual(refute("no"), {
    support: false,
    refute: { message: "no" },
  });
  assert.deepEqual(both({ message: "conflict", data: { source: 2 } }), {
    support: true,
    refute: { message: "conflict", data: { source: 2 } },
  });
  assert.deepEqual(neither(), { support: false });
});

test("refuting helpers reject values that could erase refutation on the wire", () => {
  for (const invalid of [undefined, null, "", "   ", {}, [], { message: "" }]) {
    assert.throws(() => refute(invalid), /non-empty message/);
    assert.throws(() => both(invalid), /non-empty message/);
  }
});

test("the SDK keeps initialize closed and returns method-not-found precisely", async (t) => {
  const fixture = fileURLToPath(
    new URL("../../../crates/selta-core/tests/fixtures/host.mjs", import.meta.url),
  );
  const child = spawn(process.execPath, [fixture], { stdio: ["pipe", "pipe", "pipe"] });
  t.after(async () => {
    if (child.exitCode !== null || child.signalCode !== null) return;
    const exited = new Promise((resolve) => child.once("exit", resolve));
    child.kill();
    await exited;
  });

  let nextId = 1;
  let stdout = "";
  let stderr = "";
  const waiting = new Map();
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => (stderr += chunk));
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
    for (;;) {
      const newline = stdout.indexOf("\n");
      if (newline < 0) break;
      const line = stdout.slice(0, newline);
      stdout = stdout.slice(newline + 1);
      const message = JSON.parse(line);
      waiting.get(message.id)?.(message);
      waiting.delete(message.id);
    }
  });

  const request = (method, params = {}) => {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        waiting.delete(id);
        reject(new Error(`SDK did not answer '${method}'; stderr: ${stderr}`));
      }, 2_000);
      waiting.set(id, (message) => {
        clearTimeout(timer);
        resolve(message);
      });
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
    });
  };

  const initialized = await request("initialize", {
    protocol: 1,
    server: { name: "sdk-test", version: "1" },
  });
  assert.deepEqual(Object.keys(initialized.result).sort(), ["extensions", "host"]);
  assert.deepEqual(initialized.result.extensions, [
    {
      name: "length_judge",
      determinism: "nondeterministic",
      semantic_revision: null,
      cacheable: false,
      effect_class: null,
      accepted_input: null,
      config_schema: null,
      needs: [],
      settings_schema: null,
      delta_schema: null,
    },
    {
      name: "always_pass",
      determinism: "deterministic",
      semantic_revision: "selta.test.always_pass.v1",
      cacheable: true,
      effect_class: "pure",
      accepted_input: ["str"],
      config_schema: null,
      needs: [],
      settings_schema: null,
      delta_schema: null,
    },
  ]);

  const noAssessor = await request("assess", {
    ext: "always_pass",
    config: {},
    settings: {},
    value: "x",
    path: "$",
    budget: { depth: 1 },
  });
  assert.equal(noAssessor.error.code, -32601);

  const unknown = await request("not-a-method");
  assert.equal(unknown.error.code, -32601);
  assert.match(unknown.error.message, /unknown method/);
});
