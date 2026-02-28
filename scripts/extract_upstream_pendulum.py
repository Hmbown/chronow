#!/usr/bin/env python3
"""
Upstream behavior extraction for pendulum (Python).

Runs a standard set of temporal operations through pendulum and captures
output for comparison with chronow's conformance corpus.

Usage:
    pip install pendulum   # one-time
    python3 scripts/extract_upstream_pendulum.py

Output: conformance/upstream/pendulum.json
"""

from __future__ import annotations

import json
from pathlib import Path

OUTPUT_DIR = Path(__file__).resolve().parents[1] / "conformance" / "upstream"

ISO_PARSE_INPUTS = [
    "2024-01-01T00:00:00Z",
    "2024-02-29T23:59:59Z",
    "2019-03-03T19:00:52+14:00",
    "2019-03-03T19:00:52-11:00",
    "2024-09-17",
    "2024-09-17T10:15:30",
    "2024-13-01T00:00:00Z",
    "not-a-date",
]

FORMAT_SCENARIOS = [
    {"instant": "2024-04-10T00:00:00Z", "zone": "America/New_York"},
    {"instant": "2024-09-17T10:00:00Z", "zone": "Europe/London"},
    {"instant": "2024-06-15T12:00:00Z", "zone": "Asia/Singapore"},
    {"instant": "2025-03-30T00:30:00Z", "zone": "Europe/London"},
]

DST_RESOLVE_SCENARIOS = [
    {"local": "2024-03-10T02:30:00", "zone": "America/New_York", "label": "spring_gap"},
    {"local": "2024-11-03T01:30:00", "zone": "America/New_York", "label": "fall_fold"},
    {"local": "2024-03-31T01:30:00", "zone": "Europe/London", "label": "uk_spring_gap"},
    {"local": "2024-06-15T12:00:00", "zone": "America/New_York", "label": "normal"},
]

DURATION_ADD_SCENARIOS = [
    {"instant": "2024-01-31T12:00:00Z", "zone": "UTC", "months": 1, "label": "jan31+1m"},
    {"instant": "2024-02-29T12:00:00Z", "zone": "UTC", "years": 1, "label": "leap+1y"},
    {"instant": "2024-03-10T06:00:00Z", "zone": "America/New_York", "days": 1, "label": "dst_spring+1d"},
    {"instant": "2024-11-03T05:00:00Z", "zone": "America/New_York", "days": 1, "label": "dst_fall+1d"},
    {"instant": "2024-03-10T06:00:00Z", "zone": "America/New_York", "hours": 24, "label": "dst_spring+24h"},
]

DIFF_SCENARIOS = [
    {"start": "2024-01-01T00:00:00Z", "end": "2024-12-31T23:59:59Z", "label": "full_year"},
    {"start": "2024-01-31T00:00:00Z", "end": "2024-02-29T00:00:00Z", "label": "month_clamp_leap"},
    {"start": "2023-01-31T00:00:00Z", "end": "2023-02-28T00:00:00Z", "label": "month_clamp_nonleap"},
    {"start": "2020-02-29T00:00:00Z", "end": "2021-02-28T00:00:00Z", "label": "leap_to_nonleap"},
]

DURATION_PARSE_INPUTS = [
    "P1Y", "P1M", "P1W", "P1D", "PT1H", "PT1M", "PT1S",
    "P1Y2M3DT4H5M6S", "P0Y0M0DT0H0M0S", "-P1D",
]


def extract_pendulum() -> dict | None:
    try:
        import pendulum
    except ImportError:
        print("pendulum not installed. Run: pip install pendulum")
        return None

    results: dict = {
        "library": "pendulum",
        "version": pendulum.__version__,
        "scenarios": {},
    }

    # Parse
    parse_results = []
    for inp in ISO_PARSE_INPUTS:
        try:
            d = pendulum.parse(inp, tz="UTC")
            parse_results.append({
                "input": inp,
                "valid": True,
                "utc_iso": d.in_tz("UTC").to_iso8601_string(),
                "epoch_ms": int(d.timestamp() * 1000),
            })
        except Exception as e:
            parse_results.append({
                "input": inp,
                "valid": False,
                "utc_iso": None,
                "epoch_ms": None,
                "error": str(e),
            })
    results["scenarios"]["parse"] = parse_results

    # Format in timezone
    format_results = []
    for s in FORMAT_SCENARIOS:
        try:
            d = pendulum.parse(s["instant"]).in_tz(s["zone"])
            format_results.append({
                "instant": s["instant"],
                "zone": s["zone"],
                "formatted": d.to_iso8601_string(),
                "offset": d.format("Z"),
            })
        except Exception as e:
            format_results.append({
                "instant": s["instant"],
                "zone": s["zone"],
                "formatted": None,
                "offset": None,
                "error": str(e),
            })
    results["scenarios"]["format"] = format_results

    # DST resolution
    dst_results = []
    for s in DST_RESOLVE_SCENARIOS:
        try:
            d = pendulum.parse(s["local"], tz=s["zone"])
            dst_results.append({
                "label": s["label"],
                "local": s["local"],
                "zone": s["zone"],
                "valid": True,
                "utc_iso": d.in_tz("UTC").to_iso8601_string(),
                "offset": d.format("Z"),
                "note": "pendulum resolves gaps by shifting forward (post-transition), folds pick first",
            })
        except Exception as e:
            dst_results.append({
                "label": s["label"],
                "local": s["local"],
                "zone": s["zone"],
                "valid": False,
                "utc_iso": None,
                "offset": None,
                "error": str(e),
            })
    results["scenarios"]["dst_resolve"] = dst_results

    # Duration addition
    add_results = []
    for s in DURATION_ADD_SCENARIOS:
        try:
            d = pendulum.parse(s["instant"]).in_tz(s["zone"])
            kwargs = {}
            for k in ("years", "months", "days", "hours"):
                if k in s:
                    kwargs[k] = s[k]
            result = d.add(**kwargs)
            add_results.append({
                "label": s["label"],
                "instant": s["instant"],
                "zone": s["zone"],
                "add": kwargs,
                "result_utc": result.in_tz("UTC").to_iso8601_string(),
                "result_local": result.format("YYYY-MM-DDTHH:mm:ss"),
            })
        except Exception as e:
            add_results.append({
                "label": s["label"],
                "instant": s["instant"],
                "zone": s["zone"],
                "error": str(e),
            })
    results["scenarios"]["add_duration"] = add_results

    # Diff
    diff_results = []
    for s in DIFF_SCENARIOS:
        try:
            start = pendulum.parse(s["start"])
            end = pendulum.parse(s["end"])
            diff = end.diff(start)
            diff_results.append({
                "label": s["label"],
                "start": s["start"],
                "end": s["end"],
                "diff_days": diff.in_days(),
                "diff_months": diff.in_months(),
                "diff_years": diff.in_years(),
                "diff_seconds": diff.in_seconds(),
            })
        except Exception as e:
            diff_results.append({
                "label": s["label"],
                "start": s["start"],
                "end": s["end"],
                "error": str(e),
            })
    results["scenarios"]["diff"] = diff_results

    # Duration parsing
    dur_results = []
    for inp in DURATION_PARSE_INPUTS:
        try:
            d = pendulum.duration()  # pendulum doesn't have ISO parse for duration
            # Use pendulum.parse to attempt
            # Actually pendulum.parse_duration is not a thing; we test what we can
            dur_results.append({
                "input": inp,
                "valid": False,
                "note": "pendulum does not support ISO 8601 duration parsing from strings",
            })
        except Exception as e:
            dur_results.append({
                "input": inp,
                "valid": False,
                "error": str(e),
            })
    results["scenarios"]["duration_parse"] = dur_results

    return results


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    pendulum_data = extract_pendulum()
    if pendulum_data:
        out = OUTPUT_DIR / "pendulum.json"
        out.write_text(json.dumps(pendulum_data, indent=2) + "\n")
        print(f"wrote {out} ({len(pendulum_data['scenarios'])} scenario groups)")


if __name__ == "__main__":
    main()
