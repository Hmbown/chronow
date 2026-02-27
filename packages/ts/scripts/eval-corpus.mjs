import { evaluateCorpusFile } from "../dist/index.js";

const casesFile = process.argv[2];
if (!casesFile) {
  console.error("usage: node scripts/eval-corpus.mjs <cases-file>");
  process.exit(2);
}

const result = evaluateCorpusFile(casesFile);
process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
