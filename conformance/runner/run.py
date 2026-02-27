#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

ROOT = Path(__file__).resolve().parents[2]
CASES_DIR = ROOT / "conformance" / "cases"
TMP_DIR = ROOT / "conformance" / "runner" / ".tmp"
CHRONOW_BIN = ROOT / "target" / "debug" / "chronow"


def canonical(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def load_cases(paths: Iterable[Path]) -> List[Dict[str, Any]]:
    merged: List[Dict[str, Any]] = []
    for path in paths:
        if path.name.startswith("_"):
            continue
        payload = json.loads(path.read_text())
        merged.extend(payload["cases"])
    return merged


def write_merged_cases(cases: List[Dict[str, Any]]) -> Path:
    TMP_DIR.mkdir(parents=True, exist_ok=True)
    target = TMP_DIR / "all_cases.json"
    target.write_text(json.dumps({"cases": cases}, indent=2, sort_keys=True) + "\n")
    return target


def run_cmd(cmd: List[str], env: Dict[str, str] | None = None) -> Dict[str, Any]:
    completed = subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)
    return json.loads(completed.stdout)


def adapter_rust(cases_file: Path) -> Dict[str, Any]:
    return run_cmd([str(CHRONOW_BIN), "eval-corpus", "--cases-file", str(cases_file)])


def adapter_ts(cases_file: Path) -> Dict[str, Any]:
    env = os.environ.copy()
    env["CHRONOW_BIN"] = str(CHRONOW_BIN)
    subprocess.run(
        ["npm", "run", "--prefix", str(ROOT / "packages" / "ts"), "build"],
        check=True,
        capture_output=True,
        text=True,
        env=env,
    )
    cmd = [
        "node",
        str(ROOT / "packages" / "ts" / "scripts" / "eval-corpus.mjs"),
        str(cases_file),
    ]
    return run_cmd(cmd, env=env)


def adapter_python(cases_file: Path) -> Dict[str, Any]:
    cmd = ["python3", str(ROOT / "packages" / "python" / "scripts" / "eval_corpus.py"), str(cases_file)]
    env = os.environ.copy()
    env["CHRONOW_BIN"] = str(CHRONOW_BIN)
    py_path = str(ROOT / "packages" / "python")
    env["PYTHONPATH"] = py_path if "PYTHONPATH" not in env else f"{py_path}:{env['PYTHONPATH']}"
    return run_cmd(cmd, env=env)


def to_map(result_payload: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    return {item["id"]: item["response"] for item in result_payload["results"]}


def compare_adapter(
    name: str,
    case_list: List[Dict[str, Any]],
    actual_map: Dict[str, Dict[str, Any]],
) -> List[Tuple[str, str]]:
    mismatches: List[Tuple[str, str]] = []

    for case in case_list:
        cid = case["id"]
        expected = case["expected"]
        actual = actual_map.get(cid)
        if actual is None:
            mismatches.append((cid, "missing result"))
            continue
        if canonical(expected) != canonical(actual):
            mismatches.append((cid, "response mismatch"))

    return mismatches


def compare_parity(adapter_maps: Dict[str, Dict[str, Dict[str, Any]]]) -> Dict[str, List[str]]:
    discrepancies: Dict[str, List[str]] = defaultdict(list)
    names = list(adapter_maps.keys())
    if len(names) < 2:
        return discrepancies

    baseline = names[0]
    for cid, baseline_value in adapter_maps[baseline].items():
        base_norm = canonical(baseline_value)
        for other in names[1:]:
            other_value = adapter_maps[other].get(cid)
            if other_value is None:
                discrepancies[cid].append(f"missing in {other}")
                continue
            if canonical(other_value) != base_norm:
                discrepancies[cid].append(f"{baseline} != {other}")
    return discrepancies


def main() -> None:
    parser = argparse.ArgumentParser(description="Run Chronow conformance matrix")
    parser.add_argument(
        "--matrix",
        nargs="+",
        default=["rust", "ts", "python"],
        choices=["rust", "ts", "python"],
    )
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    if not CHRONOW_BIN.exists():
        raise SystemExit(
            f"chronow binary not found at {CHRONOW_BIN}; run `cargo build -p chronow-cli` first"
        )

    case_files = sorted(CASES_DIR.glob("*.json"))
    if not case_files:
        raise SystemExit(f"no conformance case files found under {CASES_DIR}")

    cases = load_cases(case_files)
    merged_path = write_merged_cases(cases)

    adapters = {
        "rust": adapter_rust,
        "ts": adapter_ts,
        "python": adapter_python,
    }

    adapter_maps: Dict[str, Dict[str, Dict[str, Any]]] = {}
    adapter_failures: Dict[str, List[Tuple[str, str]]] = {}

    for name in args.matrix:
        payload = adapters[name](merged_path)
        actual_map = to_map(payload)
        adapter_maps[name] = actual_map
        adapter_failures[name] = compare_adapter(name, cases, actual_map)

    parity = compare_parity(adapter_maps)

    print(f"cases: {len(cases)}")
    for name in args.matrix:
        mismatches = adapter_failures[name]
        print(f"adapter={name} mismatches={len(mismatches)}")
        for cid, reason in mismatches[:10]:
            print(f"  - {cid}: {reason}")

    print(f"cross-language parity mismatches={len(parity)}")
    for cid, reasons in list(parity.items())[:10]:
        print(f"  - {cid}: {', '.join(reasons)}")

    has_failures = any(adapter_failures[name] for name in args.matrix) or bool(parity)
    if has_failures and args.strict:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
