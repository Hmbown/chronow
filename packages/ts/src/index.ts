import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import type {
  ChronowOptions,
  CorpusCase,
  EngineResponse,
  EvalCorpusResult,
  JsonObject,
} from "./types.js";

type WasmModule = {
  evaluate_json_wasm: (requestJson: string) => string;
};

type MaybeWasmModule = {
  evaluate_json_wasm?: unknown;
};

let wasmModule: WasmModule | null = null;
let wasmLoadAttempted = false;

export async function initWasm(): Promise<boolean> {
  if (wasmModule) return true;
  if (wasmLoadAttempted) return false;
  wasmLoadAttempted = true;

  try {
    // @ts-ignore - WASM module may not exist at compile time
    const mod = (await import(
      /* webpackIgnore: true */ "../wasm/chronow_core.js"
    )) as MaybeWasmModule;
    if (typeof mod.evaluate_json_wasm === "function") {
      const evaluateJsonWasm = mod.evaluate_json_wasm as (requestJson: string) => string;
      wasmModule = {
        evaluate_json_wasm: evaluateJsonWasm,
      };
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

function resolveChronowBin(options?: ChronowOptions): string {
  if (options?.chronowBin) {
    return options.chronowBin;
  }

  if (process.env.CHRONOW_BIN && process.env.CHRONOW_BIN.trim().length > 0) {
    return process.env.CHRONOW_BIN;
  }

  const local = path.resolve(process.cwd(), "target", "debug", "chronow");
  if (existsSync(local)) {
    return local;
  }

  return "chronow";
}

function runChronow(args: string[], options?: ChronowOptions): string {
  const bin = resolveChronowBin(options);
  return execFileSync(bin, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function evaluateViaCli(request: JsonObject, options?: ChronowOptions): EngineResponse {
  const stdout = runChronow(["eval", "--request", JSON.stringify(request)], options);
  return JSON.parse(stdout) as EngineResponse;
}

function evaluateViaWasm(request: JsonObject): EngineResponse {
  if (!wasmModule) {
    throw new Error("WASM module not initialized; call initWasm() first");
  }
  const raw = wasmModule.evaluate_json_wasm(JSON.stringify(request));
  return JSON.parse(raw) as EngineResponse;
}

export function evaluate(request: JsonObject, options?: ChronowOptions): EngineResponse {
  if (wasmModule && !options?.forceCli) {
    return evaluateViaWasm(request);
  }
  return evaluateViaCli(request, options);
}

export function evaluateCorpusFile(casesFile: string, options?: ChronowOptions): EvalCorpusResult {
  // WASM path: read and process in-process
  if (wasmModule && !options?.forceCli) {
    const content = readFileSync(casesFile, "utf8");
    const parsed = JSON.parse(content);
    const cases: CorpusCase[] = Array.isArray(parsed) ? parsed : parsed.cases;
    return evaluateCorpusInProcess(cases);
  }

  const stdout = runChronow(["eval-corpus", "--cases-file", casesFile], options);
  return JSON.parse(stdout) as EvalCorpusResult;
}

function evaluateCorpusInProcess(cases: CorpusCase[]): EvalCorpusResult {
  const results = cases.map((c) => ({
    id: c.id,
    response: evaluate(c.request),
  }));
  return { results, count: results.length };
}

export function evaluateCorpus(cases: CorpusCase[], options?: ChronowOptions): EvalCorpusResult {
  if (wasmModule && !options?.forceCli) {
    return evaluateCorpusInProcess(cases);
  }

  const tmp = mkdtempSync(path.join(os.tmpdir(), "chronow-ts-"));
  const casesFile = path.join(tmp, "cases.json");
  writeFileSync(casesFile, JSON.stringify({ cases }), "utf8");
  try {
    return evaluateCorpusFile(casesFile, options);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

export * from "./types.js";
