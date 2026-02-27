#!/usr/bin/env python3
from __future__ import annotations

import json
import sys

from chronow import evaluate_corpus_file


if len(sys.argv) != 2:
    print("usage: python scripts/eval_corpus.py <cases-file>", file=sys.stderr)
    raise SystemExit(2)

result = evaluate_corpus_file(sys.argv[1])
print(json.dumps(result, indent=2, sort_keys=True))
