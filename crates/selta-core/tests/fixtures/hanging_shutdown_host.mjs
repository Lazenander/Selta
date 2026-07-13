import { writeFile } from "node:fs/promises";
import readline from "node:readline";

const [marker] = process.argv.slice(2);
if (!marker) throw new Error("marker path is required");

const input = readline.createInterface({ input: process.stdin, terminal: false });
const reply = (id, result) => {
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id, result }) + "\n");
};

input.on("line", (line) => {
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    reply(message.id, {
      host: { name: "hanging-shutdown-host", version: "0.0.1" },
      extensions: [],
    });
  } else if (message.method === "shutdown") {
    reply(message.id, {});
    setTimeout(() => {
      void writeFile(marker, "host survived shutdown\n", "utf8");
    }, 3_000);
  }
});
