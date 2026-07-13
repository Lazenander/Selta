import { writeFile } from "node:fs/promises";

const [marker] = process.argv.slice(2);
if (!marker) throw new Error("marker path is required");

await new Promise((resolve) => setTimeout(resolve, 200));
await writeFile(marker, "child survived timeout\n", "utf8");
