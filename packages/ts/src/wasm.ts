import type { EngineResponse, JsonObject } from "./types.js";

export type WasmEvaluator = {
  evaluate_json_wasm: (requestJson: string) => string;
};

export async function loadWasmEvaluator(): Promise<WasmEvaluator> {
  // WASM artifact is generated via:
  //   wasm-pack build crates/core --target nodejs --out-dir ../../packages/ts/wasm
  const mod: any = await import(/* webpackIgnore: true */ "../wasm/chronow_core.js");
  return mod as WasmEvaluator;
}

export async function evaluateViaWasm(request: JsonObject): Promise<EngineResponse> {
  const wasm = await loadWasmEvaluator();
  const raw = wasm.evaluate_json_wasm(JSON.stringify(request));
  return JSON.parse(raw) as EngineResponse;
}
