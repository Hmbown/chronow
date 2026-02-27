import fs from "node:fs";
import path from "node:path";
import { evaluateCorpusFile } from "../dist/index.js";

const root = path.resolve(process.cwd(), "..", "..");
const allCases = path.join(root, "conformance", "runner", ".tmp", "all_cases.json");

if (!fs.existsSync(allCases)) {
  console.error(`missing ${allCases}; run conformance runner to materialize merged corpus`);
  process.exit(1);
}

const output = evaluateCorpusFile(allCases);
process.stdout.write(`${JSON.stringify(output, null, 2)}\n`);
