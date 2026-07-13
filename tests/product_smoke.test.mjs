import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { promisify } from "node:util";

const execute = promisify(execFile);
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const suffix = process.platform === "win32" ? ".exe" : "";
const seltad = join(root, "target", "debug", `seltad${suffix}`);
const selta = join(root, "target", "debug", `selta${suffix}`);

function waitForServer(child, stderr) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`seltad did not listen within 10 seconds\n${stderr.text}`));
    }, 10_000);
    const onExit = (code, signal) => {
      clearTimeout(timeout);
      reject(new Error(`seltad exited before listening (${code ?? signal})\n${stderr.text}`));
    };
    child.once("exit", onExit);
    child.stderr.on("data", (chunk) => {
      stderr.text += chunk;
      // Wait for the terminating whitespace as stderr may split the address
      // immediately after the colon on Windows pipes.
      const match = stderr.text.match(/listening on http:\/\/(127\.0\.0\.1:\d+)\s/);
      if (match) {
        clearTimeout(timeout);
        child.off("exit", onExit);
        resolve(`http://${match[1]}`);
      }
    });
  });
}

async function stop(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  const stopped = new Promise((resolve) => {
    const timeout = setTimeout(() => resolve(false), 3_000);
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve(true);
    });
  });
  child.kill();
  if (
    !(await stopped) &&
    child.exitCode === null &&
    child.signalCode === null
  ) {
    const exited = new Promise((resolve) => child.once("exit", resolve));
    child.kill("SIGKILL");
    await exited;
  }
}

test("native daemon and CLI round-trip over TCP and SQLite", async (t) => {
  const scratch = await mkdtemp(join(tmpdir(), "selta-w1-"));
  const work = join(scratch, "données-資料");
  await mkdir(work);
  await writeFile(
    join(work, "selta.toml"),
    'listen = "127.0.0.1:0"\ndata = "./catalog-資料"\nstorage = "sqlite"\n',
    "utf8",
  );

  const child = spawn(seltad, ["selta.toml"], {
    cwd: work,
    stdio: ["ignore", "ignore", "pipe"],
    windowsHide: true,
  });
  child.stderr.setEncoding("utf8");
  const stderr = { text: "" };
  t.after(async () => {
    await stop(child);
    await rm(scratch, { recursive: true, force: true });
  });

  const server = await waitForServer(child, stderr);
  const run = (...args) =>
    execute(selta, ["--server", server, ...args], {
      cwd: work,
      encoding: "utf8",
      windowsHide: true,
    });

  await run("pool", "create", "windows_smoke");
  const listed = await run("pool", "list");
  assert.match(listed.stdout, /windows_smoke/);

  const schema = join(work, "schéma.json");
  const value = join(work, "réponse.json");
  await writeFile(
    schema,
    JSON.stringify({
      type: "object",
      fields: { message: { type: "str" } },
    }),
    "utf8",
  );
  await writeFile(value, JSON.stringify({ message: "你好, Windows" }), "utf8");
  await run("schema", "put", "windows_smoke", "answer", schema);
  const verified = await run(
    "verify",
    "windows_smoke",
    "answer",
    value,
    "--json",
  );
  assert.equal(JSON.parse(verified.stdout).verdict, "pass");
});
