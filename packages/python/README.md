# chronow

Python bindings for the Chronow temporal engine.

This package is a thin wrapper around the `chronow` CLI so the Python adapter stays byte-identical with the Rust and TypeScript adapters.

## Install

```bash
pip install chronow
```

You also need the `chronow` binary on your `$PATH` (or set `CHRONOW_BIN=/path/to/chronow`).

## Usage

```python
from chronow import evaluate

resp = evaluate({"op": "now", "zone": "America/Chicago"})
print(resp["value"]["zoned"])
```
