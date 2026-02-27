from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Dict


JsonDict = Dict[str, Any]


def _resolve_chronow_bin(chronow_bin: str | None = None) -> str:
    if chronow_bin:
        return chronow_bin

    env_bin = os.environ.get("CHRONOW_BIN")
    if env_bin:
        return env_bin

    local_bin = Path.cwd() / "target" / "debug" / "chronow"
    if local_bin.exists():
        return str(local_bin)

    return "chronow"


def _run(args: list[str], chronow_bin: str | None = None) -> str:
    bin_path = _resolve_chronow_bin(chronow_bin)
    completed = subprocess.run(
        [bin_path, *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout


def evaluate(request: JsonDict, chronow_bin: str | None = None) -> JsonDict:
    stdout = _run(["eval", "--request", json.dumps(request)], chronow_bin)
    return json.loads(stdout)


def evaluate_corpus_file(cases_file: str, chronow_bin: str | None = None) -> JsonDict:
    stdout = _run(["eval-corpus", "--cases-file", cases_file], chronow_bin)
    return json.loads(stdout)
