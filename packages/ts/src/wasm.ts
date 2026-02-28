import type { EngineResponse, JsonObject } from "./types.js";

export type WasmEvaluator = {
  evaluate_json_wasm: (requestJson: string) => string;
};

export async function loadWasmEvaluator(): Promise<WasmEvaluator> {
  // WASM artifact is generated via:
  //   wasm-pack build crates/core --target nodejs --out-dir ../../packages/ts/wasm
  const mod = (await import(
    /* webpackIgnore: true */ "../wasm/chronow_core.js"
  )) as Partial<WasmEvaluator>;
  if (typeof mod.evaluate_json_wasm !== "function") {
    throw new Error("WASM module missing evaluate_json_wasm export");
  }
  const evaluateJsonWasm = mod.evaluate_json_wasm as (requestJson: string) => string;
  return {
    evaluate_json_wasm: evaluateJsonWasm,
  };
}

export async function evaluateViaWasm(request: JsonObject): Promise<EngineResponse> {
  const wasm = await loadWasmEvaluator();
  const raw = wasm.evaluate_json_wasm(JSON.stringify(request));
  return JSON.parse(raw) as EngineResponse;
}
