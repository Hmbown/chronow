# Chronow

Chronow is an agent-native, deterministic temporal engine with a conformance-first workflow.

## Components

- `crates/core`: Rust temporal engine.
- `cli`: `chronow` CLI for parsing, conversion, and recurrence previews.
- `packages/ts`: TypeScript package (WASM-ready + CLI adapter).
- `packages/python`: Python binding/package.
- `conformance/cases`: Canonical conformance corpus JSON.
- `conformance/runner`: Cross-language conformance matrix runner.
- `spec/temporal-contract.md`: Normative behavior contract.

## Deterministic conflict policy

For ambiguous/non-existent local times:
- `compatible`: ambiguous -> earlier, nonexistent -> shift forward
- `earlier`: ambiguous -> earlier, nonexistent -> shift backward
- `later`: ambiguous -> later, nonexistent -> shift forward
- `reject`: fail with explicit error

## Quick start

```bash
cargo run -p chronow-cli -- parse --input 2024-04-10T00:00:00Z
cargo run -p chronow-cli -- convert --input 2024-04-10T00:00:00Z --zone America/New_York
cargo run -p chronow-cli -- recur --start-local 2026-03-01T09:00:00 --zone America/New_York --freq daily --count 5
```

Run full conformance matrix:

```bash
python3 conformance/runner/run.py --matrix rust ts python
```

Regenerate corpus:

```bash
python3 scripts/generate_conformance_cases.py --chronow-bin target/debug/chronow
```
