# Conformance Runner

Run all adapters against merged canonical cases:

```bash
python3 conformance/runner/run.py --matrix rust ts python --strict
```

What it checks:
- Adapter output == case `expected`
- Cross-language adapter output parity

Temporary merged corpus is written to `conformance/runner/.tmp/all_cases.json`.
