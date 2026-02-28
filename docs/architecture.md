# Architecture, Tradeoffs, and Known Gaps

## Architecture
- Core runtime: Rust library in `crates/core` with a typed request/response contract.
- Execution surface: `chronow` CLI in `cli` for direct usage and adapter bridging.
- Cross-language bindings:
  - TypeScript package in `packages/ts` (CLI bridge + WASM export path).
  - Python package in `packages/python` (CLI bridge).
- Contract enforcement: canonical JSON corpus in `conformance/cases` and matrix runner in `conformance/runner`.

## Tradeoffs
- Chosen: strict determinism and parity over adapter-native implementation differences.
- Chosen: explicit DST disambiguation modes over implicit library defaults.
- Chosen: bounded NL grammar over fuzzy parsing to keep agent behavior predictable.
- Chosen: conformance corpus generation from deterministic seeds + engine response materialization to scale to 865 cases quickly.

## Known gaps
- Upstream extraction covers date-fns, dayjs, luxon, and pendulum; arrow, chrono, jiff, and nodatime remain unextracted.
- TS/Python adapters currently bridge via CLI for parity; native in-process engines are not implemented yet.
- WASM bridge is implemented via exported Rust function (`evaluate_json_wasm`) but packaging of prebuilt wasm artifacts is not automated in CI.
- Benchmark numbers are from local debug builds; release/CI hardware benchmarking should be added for publish-quality performance claims.
