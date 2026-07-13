import { readFile } from "node:fs/promises";

const [path] = process.argv.slice(2);
if (!path) throw new Error("input path is required");

const payload = await readFile(path, "utf8");
if (payload !== "portable payload") {
  console.error(`unexpected command input: ${JSON.stringify(payload)}`);
  process.exitCode = 1;
}
