# Benchmarks and Correctness

## Environment
- Host: local macOS dev machine
- Build: `cargo build -p chronow-cli` (debug)
- Corpus size: 865 cases

## Correctness
Full matrix run:
```bash
python3 conformance/runner/run.py --matrix rust ts python --strict
```
Result:
- Rust mismatches: 0
- TypeScript mismatches: 0
- Python mismatches: 0
- Cross-language parity mismatches: 0

## Benchmark snapshots
Measured with repeated local runs:
- `chronow eval-corpus` over 865 cases (Rust adapter):
  - runs: `0.1869s, 0.1855s, 0.1839s, 0.1888s, 0.1899s`
  - mean: `0.1870s`
- Full matrix (`rust + ts + python`) strict run:
  - runs: `1.0880s, 1.0770s, 1.0804s`
  - mean: `1.0818s`

## Notes
- TypeScript and Python adapters intentionally delegate to the same deterministic core contract for parity guarantees.
- Release benchmarks should be re-run under `--release` and CI hardware for publish-quality comparisons.
