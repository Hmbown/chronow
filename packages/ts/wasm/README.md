# Chronow WASM Bridge

This package is WASM-ready. Build artifacts are expected in `wasm/pkg`.

Suggested build command:

```bash
wasm-pack build ../../crates/core --target nodejs --features wasm --out-dir ../packages/ts/wasm/pkg
```

The TypeScript package can dynamically load `wasm/pkg/chronow_wasm.js` via `src/wasm.ts`.
