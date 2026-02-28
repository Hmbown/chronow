#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BIN = ROOT / "target" / "debug" / "chronow"
CASES_DIR = ROOT / "conformance" / "cases"

DATE_FNS_PARSE_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/parseISO/test.ts",
    "note": "ISO parsing, timezone offsets, invalid date handling, DST gap compatibility",
}

DATE_FNS_FORMAT_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/formatISO/test.ts",
    "note": "ISO formatting shape and offset rendering",
}

DATE_FNS_BUSINESS_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/addBusinessDays/test.ts",
    "note": "business-day semantics and weekend skipping",
}

DATE_FNS_INTERVAL_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/eachDayOfInterval/test.ts",
    "note": "interval inclusivity and directional day stepping",
}

AVAILABLE_UPSTREAMS = [
    "date-fns",
    "dayjs",
    "luxon",
    "pendulum",
]

UNAVAILABLE_UPSTREAMS = [
    "arrow",
    "dateparser",
    "chrono",
    "jiff",
    "nodatime",
]

MAJOR_DST_ZONES = [
    "America/New_York",
    "America/Los_Angeles",
    "America/Chicago",
    "America/Denver",
    "America/Toronto",
    "America/Halifax",
    "America/Anchorage",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Madrid",
    "Europe/Rome",
    "Europe/Amsterdam",
    "Europe/Stockholm",
    "Europe/Vienna",
    "Australia/Sydney",
    "Australia/Melbourne",
    "Australia/Adelaide",
    "Australia/Hobart",
    "Pacific/Auckland",
]

GENERATED_AT = "1970-01-01T00:00:00+00:00"


def iso_local(dt: datetime) -> str:
    return dt.strftime("%Y-%m-%dT%H:%M:%S")


def parse_response(raw: str) -> Dict[str, Any]:
    return json.loads(raw)


def eval_request(bin_path: Path, request: Dict[str, Any]) -> Dict[str, Any]:
    cmd = [str(bin_path), "eval", "--request", json.dumps(request)]
    completed = subprocess.run(cmd, check=True, capture_output=True, text=True)
    return parse_response(completed.stdout)


@dataclass
class CaseFactory:
    suite_prefix: str
    counter: int = 0

    def make(
        self,
        *,
        category: str,
        description: str,
        request: Dict[str, Any],
        expected: Dict[str, Any],
        sources: List[Dict[str, str]],
    ) -> Dict[str, Any]:
        self.counter += 1
        return {
            "id": f"{self.suite_prefix}.{self.counter:04d}",
            "category": category,
            "description": description,
            "request": request,
            "expected": expected,
            "sources": sources,
        }


def find_transitions(zone: str, year: int) -> List[Tuple[datetime, timedelta, timedelta]]:
    tz = ZoneInfo(zone)
    start = datetime(year, 1, 1, tzinfo=UTC)
    end = datetime(year + 1, 1, 1, tzinfo=UTC)
    step = timedelta(hours=6)

    out: List[Tuple[datetime, timedelta, timedelta]] = []
    prev_t = start
    prev_offset = prev_t.astimezone(tz).utcoffset() or timedelta(0)

    t = start + step
    while t <= end:
        current_offset = t.astimezone(tz).utcoffset() or timedelta(0)
        if current_offset != prev_offset:
            lo = prev_t
            hi = t
            while hi - lo > timedelta(minutes=1):
                mid = lo + (hi - lo) / 2
                mid_offset = mid.astimezone(tz).utcoffset() or timedelta(0)
                if mid_offset == prev_offset:
                    lo = mid
                else:
                    hi = mid
            transition = hi
            out.append((transition, prev_offset, current_offset))
            prev_offset = current_offset

        prev_t = t
        t += step

    return out


def merge_cases(files: Iterable[Path]) -> List[Dict[str, Any]]:
    merged: List[Dict[str, Any]] = []
    for path in files:
        payload = json.loads(path.read_text())
        if "cases" not in payload:
            continue
        merged.extend(payload["cases"])
    return merged


def generate_iso_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("iso")
    cases: List[Dict[str, Any]] = []

    parse_inputs = [
        "2024-01-01T00:00:00Z",
        "2024-02-29T23:59:59Z",
        "2019-03-03T19:00:52+14:00",
        "2019-03-03T19:00:52-11:00",
        "2024-09-17",
        "2024-09-17T10:15:30",
        "2024-13-01T00:00:00Z",
        "not-a-date",
        "2024-03-10T02:00:00",
        "2024-11-03T01:30:00",
    ]

    for item in parse_inputs:
        request = {"op": "parse_instant", "input": item}
        expected = eval_request(bin_path, request)
        cases.append(
            factory.make(
                category="parse_instant",
                description=f"Parse canonical ISO/RFC3339 input '{item}'",
                request=request,
                expected=expected,
                sources=[DATE_FNS_PARSE_SOURCE],
            )
        )

    format_instants = [
        "2024-04-10T00:00:00Z",
        "2024-09-17T10:00:00Z",
        "2019-03-03T19:00:52Z",
        "2025-10-26T00:30:00Z",
        "2025-03-30T00:30:00Z",
    ]

    format_zones = [
        "America/New_York",
        "Europe/London",
        "Asia/Singapore",
        "Pacific/Auckland",
        "Australia/Sydney",
    ]

    formats = ["extended", "basic", "date", "time"]
    for instant in format_instants:
        for zone in format_zones:
            for fmt in formats:
                request = {
                    "op": "format_instant",
                    "instant": instant,
                    "zone": zone,
                    "format": fmt,
                }
                expected = eval_request(bin_path, request)
                cases.append(
                    factory.make(
                        category="format_instant",
                        description=f"Format {instant} in {zone} using {fmt}",
                        request=request,
                        expected=expected,
                        sources=[DATE_FNS_FORMAT_SOURCE],
                    )
                )

    return {
        "suite": "iso_rfc3339",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_PARSE_SOURCE, DATE_FNS_FORMAT_SOURCE],
        "cases": cases,
    }


def generate_timezone_suite(bin_path: Path, year: int) -> Dict[str, Any]:
    factory = CaseFactory("tz")
    cases: List[Dict[str, Any]] = []

    for zone in MAJOR_DST_ZONES:
        transitions = find_transitions(zone, year)
        if len(transitions) < 2:
            continue

        for index, (transition_utc, before, after) in enumerate(transitions[:2], start=1):
            before_local = (transition_utc + before - timedelta(minutes=30)).replace(tzinfo=None)
            after_local = (transition_utc + after + timedelta(minutes=30)).replace(tzinfo=None)

            for label, local in (("pre", before_local), ("post", after_local)):
                request = {
                    "op": "resolve_local",
                    "local": iso_local(local),
                    "zone": zone,
                    "disambiguation": "compatible",
                }
                expected = eval_request(bin_path, request)
                cases.append(
                    factory.make(
                        category="resolve_local",
                        description=f"{zone} transition {index} {label} window resolves deterministically",
                        request=request,
                        expected=expected,
                        sources=[DATE_FNS_PARSE_SOURCE],
                    )
                )

            delta = after - before
            if delta > timedelta(0):
                # Gap: local times do not exist.
                local_gap_start = (transition_utc + before).replace(tzinfo=None)
                local_gap_mid = local_gap_start + delta / 2
                for policy in ("compatible", "later", "earlier", "reject"):
                    request = {
                        "op": "resolve_local",
                        "local": iso_local(local_gap_mid),
                        "zone": zone,
                        "disambiguation": policy,
                    }
                    expected = eval_request(bin_path, request)
                    cases.append(
                        factory.make(
                            category="resolve_local_gap",
                            description=f"{zone} gap disambiguation={policy}",
                            request=request,
                            expected=expected,
                            sources=[DATE_FNS_PARSE_SOURCE],
                        )
                    )
            elif delta < timedelta(0):
                # Fold: local times are repeated.
                gain = before - after
                fold_start = (transition_utc + after).replace(tzinfo=None)
                local_fold_mid = fold_start + gain / 2
                for policy in ("compatible", "earlier", "later", "reject"):
                    request = {
                        "op": "resolve_local",
                        "local": iso_local(local_fold_mid),
                        "zone": zone,
                        "disambiguation": policy,
                    }
                    expected = eval_request(bin_path, request)
                    cases.append(
                        factory.make(
                            category="resolve_local_fold",
                            description=f"{zone} fold disambiguation={policy}",
                            request=request,
                            expected=expected,
                            sources=[DATE_FNS_PARSE_SOURCE],
                        )
                    )

            add_request_calendar = {
                "op": "add_duration",
                "start": transition_utc.strftime("%Y-%m-%dT%H:%M:%SZ"),
                "zone": zone,
                "arithmetic": "calendar",
                "duration": {"days": 1},
                "disambiguation": "compatible",
            }
            add_request_absolute = {
                "op": "add_duration",
                "start": transition_utc.strftime("%Y-%m-%dT%H:%M:%SZ"),
                "zone": zone,
                "arithmetic": "absolute",
                "duration": {"hours": 24},
                "disambiguation": "compatible",
            }

            for req, label in (
                (add_request_calendar, "calendar"),
                (add_request_absolute, "absolute"),
            ):
                expected = eval_request(bin_path, req)
                cases.append(
                    factory.make(
                        category="add_duration",
                        description=f"{zone} transition {index} {label} arithmetic",
                        request=req,
                        expected=expected,
                        sources=[DATE_FNS_PARSE_SOURCE],
                    )
                )

    return {
        "suite": "timezone_dst",
        "generated_at": GENERATED_AT,
        "major_zones": MAJOR_DST_ZONES,
        "upstream_basis": [DATE_FNS_PARSE_SOURCE],
        "cases": cases,
    }


def generate_recurrence_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("rec")
    cases: List[Dict[str, Any]] = []

    recurrence_requests = [
        {
            "op": "recurrence_preview",
            "start_local": "2026-01-05T09:00:00",
            "zone": "America/New_York",
            "rule": {"frequency": "daily", "interval": 1, "count": 10},
            "business_calendar": {
                "exclude_weekends": False,
                "holidays": [],
            },
            "disambiguation": "compatible",
        },
        {
            "op": "recurrence_preview",
            "start_local": "2026-01-05T09:00:00",
            "zone": "America/New_York",
            "rule": {"frequency": "daily", "interval": 1, "count": 10},
            "business_calendar": {
                "exclude_weekends": True,
                "holidays": ["2026-01-19"],
            },
            "disambiguation": "compatible",
        },
        {
            "op": "recurrence_preview",
            "start_local": "2026-03-01T08:30:00",
            "zone": "Europe/London",
            "rule": {
                "frequency": "weekly",
                "interval": 1,
                "count": 12,
                "by_weekdays": ["monday", "wednesday", "friday"],
            },
            "business_calendar": {
                "exclude_weekends": False,
                "holidays": ["2026-03-20"],
            },
            "disambiguation": "compatible",
        },
        {
            "op": "recurrence_preview",
            "start_local": "2026-01-31T10:00:00",
            "zone": "Australia/Sydney",
            "rule": {"frequency": "monthly", "interval": 1, "count": 8},
            "business_calendar": {
                "exclude_weekends": False,
                "holidays": [],
            },
            "disambiguation": "compatible",
        },
    ]

    # Expand deterministic recurrence variants.
    zones = ["America/New_York", "Europe/Berlin", "Asia/Tokyo", "Pacific/Auckland"]
    for zone in zones:
        recurrence_requests.append(
            {
                "op": "recurrence_preview",
                "start_local": "2026-10-25T01:30:00",
                "zone": zone,
                "rule": {
                    "frequency": "weekly",
                    "interval": 2,
                    "count": 10,
                    "by_weekdays": ["sunday"],
                },
                "business_calendar": {
                    "exclude_weekends": False,
                    "holidays": [],
                },
                "disambiguation": "later",
            }
        )

        recurrence_requests.append(
            {
                "op": "recurrence_preview",
                "start_local": "2026-03-01T09:00:00",
                "zone": zone,
                "rule": {
                    "frequency": "weekly",
                    "interval": 1,
                    "count": 14,
                    "by_weekdays": ["monday", "tuesday", "wednesday", "thursday", "friday"],
                },
                "business_calendar": {
                    "exclude_weekends": True,
                    "holidays": ["2026-03-17"],
                },
                "disambiguation": "compatible",
            }
        )

    for request in recurrence_requests:
        expected = eval_request(bin_path, request)
        cases.append(
            factory.make(
                category="recurrence_preview",
                description=f"Recurrence preview in {request['zone']} ({request['rule']['frequency']})",
                request=request,
                expected=expected,
                sources=[DATE_FNS_INTERVAL_SOURCE, DATE_FNS_BUSINESS_SOURCE],
            )
        )

    return {
        "suite": "recurrence_business",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_INTERVAL_SOURCE, DATE_FNS_BUSINESS_SOURCE],
        "cases": cases,
    }


def generate_intent_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("nlp")
    cases: List[Dict[str, Any]] = []

    prompts = [
        "tomorrow at 09:30",
        "tomorrow at 21:45 in America/Los_Angeles",
        "next monday at 08:00",
        "next friday at 17:15 in Europe/London",
        "in 3 days at 10:00",
        "in 14 days at 06:30 in Asia/Singapore",
        "on 2026-03-15 at 12:30",
        "on 2026-12-31 at 23:00 in Pacific/Auckland",
        "every weekday at 09:00",
        "every weekday at 09:00 in America/New_York for 12 occurrences",
        "every tuesday at 14:00",
        "every sunday at 07:30 in Europe/Berlin for 5 occurrences",
        "every saturday at 11:00",
        "every monday at 08:15 in Australia/Sydney for 9 occurrences",
        "unsupported freeform text",
    ]

    for prompt in prompts:
        request = {
            "op": "normalize_intent",
            "input": prompt,
            "reference_local": "2026-01-10T12:00:00",
            "default_zone": "America/New_York",
        }
        expected = eval_request(bin_path, request)
        cases.append(
            factory.make(
                category="normalize_intent",
                description=f"Normalize intent: {prompt}",
                request=request,
                expected=expected,
                sources=[DATE_FNS_PARSE_SOURCE],
            )
        )

    return {
        "suite": "natural_language_intent",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_PARSE_SOURCE],
        "cases": cases,
    }


DATE_FNS_DIFF_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/intervalToDuration/test.ts",
    "note": "diff, compare, intervalToDuration, differenceInMonths, compareAsc",
}

DATE_FNS_SNAP_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/startOfMonth/test.ts",
    "note": "startOf*/endOf* operations for various units",
}

DATE_FNS_DURATION_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/formatISODuration/test.ts",
    "note": "ISO 8601 duration formatting",
}

DATE_FNS_OVERLAP_SOURCE = {
    "library": "date-fns",
    "path": "conform/upstreams/typescript/date-fns__date-fns/src/areIntervalsOverlapping/test.ts",
    "note": "areIntervalsOverlapping, isWithinInterval",
}

ZONE_INFO_SOURCE = {
    "library": "chronow",
    "path": "crates/core/src/lib.rs",
    "note": "zone_info and list_zones from scratch",
}


def generate_diff_compare_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("diff")
    cases: List[Dict[str, Any]] = []

    # diff_instants cases
    diff_pairs = [
        ("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", "UTC", "same instant"),
        ("2024-01-01T00:00:00Z", "2024-12-31T23:59:59Z", "UTC", "full year span"),
        ("2024-01-31T00:00:00Z", "2024-02-29T00:00:00Z", "UTC", "month clamping (leap year)"),
        ("2023-01-31T00:00:00Z", "2023-02-28T00:00:00Z", "UTC", "month clamping (non-leap)"),
        ("2024-03-01T00:00:00Z", "2024-01-01T00:00:00Z", "UTC", "negative diff"),
        ("2024-06-15T10:30:00Z", "2024-06-15T14:45:30Z", "UTC", "same day hour diff"),
        ("2020-02-29T00:00:00Z", "2021-02-28T00:00:00Z", "UTC", "leap to non-leap year"),
        ("2024-01-01T00:00:00Z", "2025-06-15T12:30:45Z", "UTC", "multi-year diff"),
        ("2024-03-10T06:00:00Z", "2024-03-10T08:00:00Z", "America/New_York", "across DST spring forward"),
        ("2024-11-03T05:00:00Z", "2024-11-03T07:00:00Z", "America/New_York", "across DST fall back"),
    ]

    for zone in ["UTC", "America/New_York", "Europe/London", "Asia/Tokyo"]:
        diff_pairs.extend([
            ("2024-01-15T10:00:00Z", "2024-01-20T14:30:00Z", zone, f"5 day diff in {zone}"),
            ("2024-01-01T00:00:00Z", "2024-07-01T00:00:00Z", zone, f"half year in {zone}"),
            ("2024-03-15T08:00:00Z", "2025-03-15T08:00:00Z", zone, f"exact 1 year in {zone}"),
        ])

    for start, end, zone, desc in diff_pairs:
        request = {"op": "diff_instants", "start": start, "end": end, "zone": zone}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="diff_instants",
            description=f"Diff: {desc}",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DIFF_SOURCE],
        ))

    # compare_instants cases
    compare_pairs = [
        ("2024-01-01T00:00:00Z", "2024-06-01T00:00:00Z", "a < b"),
        ("2024-06-01T00:00:00Z", "2024-01-01T00:00:00Z", "a > b"),
        ("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", "a == b"),
        ("2024-01-01T00:00:00+05:00", "2023-12-31T19:00:00Z", "same instant different offsets"),
        ("2024-02-29T12:00:00Z", "2024-03-01T12:00:00Z", "leap day vs next day"),
        ("2000-01-01T00:00:00Z", "2099-12-31T23:59:59Z", "century span"),
    ]

    for a, b, desc in compare_pairs:
        request = {"op": "compare_instants", "a": a, "b": b}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="compare_instants",
            description=f"Compare: {desc}",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DIFF_SOURCE],
        ))

    # Additional monthly boundary diffs
    monthly_diffs = [
        ("2024-01-28T00:00:00Z", "2024-02-29T00:00:00Z"),
        ("2024-01-29T00:00:00Z", "2024-02-29T00:00:00Z"),
        ("2024-01-30T00:00:00Z", "2024-02-29T00:00:00Z"),
        ("2024-03-31T00:00:00Z", "2024-04-30T00:00:00Z"),
        ("2024-05-31T00:00:00Z", "2024-06-30T00:00:00Z"),
        ("2024-01-31T00:00:00Z", "2024-03-31T00:00:00Z"),
        ("2024-08-31T00:00:00Z", "2024-09-30T00:00:00Z"),
        ("2024-02-29T00:00:00Z", "2025-02-28T00:00:00Z"),
    ]

    for start, end in monthly_diffs:
        request = {"op": "diff_instants", "start": start, "end": end, "zone": "UTC"}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="diff_instants",
            description=f"Month boundary diff {start[:10]} -> {end[:10]}",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DIFF_SOURCE],
        ))

    return {
        "suite": "diff_compare",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_DIFF_SOURCE],
        "cases": cases,
    }


def generate_snap_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("snap")
    cases: List[Dict[str, Any]] = []

    instants = [
        "2024-03-15T14:30:00Z",
        "2024-01-01T00:00:00Z",
        "2024-06-30T23:59:59Z",
        "2024-12-31T12:00:00Z",
        "2024-02-29T06:00:00Z",
    ]
    zones = ["UTC", "America/New_York", "Europe/London", "Asia/Tokyo", "Australia/Sydney"]
    units = ["day", "week", "month", "quarter", "year"]
    edges = ["start", "end"]

    for instant in instants:
        for zone in zones:
            for unit in units:
                for edge in edges:
                    request = {
                        "op": "snap_to",
                        "instant": instant,
                        "zone": zone,
                        "unit": unit,
                        "edge": edge,
                    }
                    expected = eval_request(bin_path, request)
                    cases.append(factory.make(
                        category="snap_to",
                        description=f"Snap {instant[:10]} to {unit} {edge} in {zone}",
                        request=request,
                        expected=expected,
                        sources=[DATE_FNS_SNAP_SOURCE],
                    ))

    # Week snap with custom start days
    for week_start in ["monday", "sunday"]:
        for instant in ["2024-01-03T12:00:00Z", "2024-12-30T12:00:00Z"]:
            for edge in ["start", "end"]:
                request = {
                    "op": "snap_to",
                    "instant": instant,
                    "zone": "UTC",
                    "unit": "week",
                    "edge": edge,
                    "week_starts_on": week_start,
                }
                expected = eval_request(bin_path, request)
                cases.append(factory.make(
                    category="snap_to",
                    description=f"Week snap {edge} starts={week_start}",
                    request=request,
                    expected=expected,
                    sources=[DATE_FNS_SNAP_SOURCE],
                ))

    return {
        "suite": "snap_to",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_SNAP_SOURCE],
        "cases": cases,
    }


def generate_duration_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("dur")
    cases: List[Dict[str, Any]] = []

    # parse_duration cases
    durations = [
        "P1Y", "P1M", "P1W", "P1D",
        "PT1H", "PT1M", "PT1S",
        "P1Y2M3DT4H5M6S",
        "P1Y6M",
        "P2W",
        "P0Y0M0DT0H0M0S",
        "P1MT1M",
        "P10Y",
        "PT3600S",
        "P365D",
        "P1Y1M1W1DT1H1M1S",
        "P12M",
        "PT90M",
        "P3M15DT8H",
        "P100D",
    ]

    for dur in durations:
        request = {"op": "parse_duration", "input": dur}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="parse_duration",
            description=f"Parse duration: {dur}",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DURATION_SOURCE],
        ))

    # Invalid durations
    invalid_durations = ["", "P", "PT", "not-a-duration", "1Y2M", "P1Y2M3D4H", "PxY"]
    for dur in invalid_durations:
        request = {"op": "parse_duration", "input": dur}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="parse_duration_error",
            description=f"Invalid duration: '{dur}'",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DURATION_SOURCE],
        ))

    # format_duration cases
    format_specs = [
        {"years": 1, "months": 2, "days": 3, "hours": 4, "minutes": 5, "seconds": 6},
        {"years": 0, "months": 0, "days": 0, "hours": 0, "minutes": 0, "seconds": 0},
        {"years": 1},
        {"months": 6},
        {"weeks": 2},
        {"days": 30, "hours": 12},
        {"hours": 1, "minutes": 30, "seconds": 45},
        {"years": 10, "months": 11, "days": 29, "hours": 23, "minutes": 59, "seconds": 59},
        {"weeks": 1, "days": 3},
    ]

    for spec in format_specs:
        request = {"op": "format_duration", "duration": spec}
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="format_duration",
            description=f"Format duration: {spec}",
            request=request,
            expected=expected,
            sources=[DATE_FNS_DURATION_SOURCE],
        ))

    return {
        "suite": "duration_parsing",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_DURATION_SOURCE],
        "cases": cases,
    }


def generate_interval_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("intv")
    cases: List[Dict[str, Any]] = []

    # Overlap tests
    overlap_pairs = [
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-05T00:00:00Z", "2024-01-20T00:00:00Z"), "overlapping"),
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-10T00:00:00Z", "2024-01-20T00:00:00Z"), "touching endpoints"),
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-15T00:00:00Z", "2024-01-20T00:00:00Z"), "disjoint"),
        (("2024-01-05T00:00:00Z", "2024-01-15T00:00:00Z"), ("2024-01-01T00:00:00Z", "2024-01-20T00:00:00Z"), "a inside b"),
        (("2024-01-01T00:00:00Z", "2024-01-20T00:00:00Z"), ("2024-01-05T00:00:00Z", "2024-01-15T00:00:00Z"), "b inside a"),
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), "identical"),
        (("2024-01-10T00:00:00Z", "2024-01-01T00:00:00Z"), ("2024-01-05T00:00:00Z", "2024-01-15T00:00:00Z"), "reversed a"),
    ]

    for (a_start, a_end), (b_start, b_end), desc in overlap_pairs:
        for mode in ["overlap", "contains", "gap"]:
            request = {
                "op": "interval_check",
                "interval_a": {"start": a_start, "end": a_end},
                "interval_b": {"start": b_start, "end": b_end},
                "mode": mode,
            }
            expected = eval_request(bin_path, request)
            cases.append(factory.make(
                category="interval_check",
                description=f"Interval {mode}: {desc}",
                request=request,
                expected=expected,
                sources=[DATE_FNS_OVERLAP_SOURCE],
            ))

    # Edge cases: zero-length intervals, very small gaps
    edge_intervals = [
        (("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"), ("2024-01-01T00:00:00Z", "2024-01-02T00:00:00Z"), "zero-length a"),
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-09T23:59:59Z", "2024-01-20T00:00:00Z"), "1-second overlap"),
        (("2024-01-01T00:00:00Z", "2024-01-10T00:00:00Z"), ("2024-01-10T00:00:01Z", "2024-01-20T00:00:00Z"), "1-second gap"),
    ]

    for (a_start, a_end), (b_start, b_end), desc in edge_intervals:
        for mode in ["overlap", "contains", "gap"]:
            request = {
                "op": "interval_check",
                "interval_a": {"start": a_start, "end": a_end},
                "interval_b": {"start": b_start, "end": b_end},
                "mode": mode,
            }
            expected = eval_request(bin_path, request)
            cases.append(factory.make(
                category="interval_check",
                description=f"Interval edge {mode}: {desc}",
                request=request,
                expected=expected,
                sources=[DATE_FNS_OVERLAP_SOURCE],
            ))

    return {
        "suite": "interval_check",
        "generated_at": GENERATED_AT,
        "upstream_basis": [DATE_FNS_OVERLAP_SOURCE],
        "cases": cases,
    }


def generate_zone_info_suite(bin_path: Path) -> Dict[str, Any]:
    factory = CaseFactory("zone")
    cases: List[Dict[str, Any]] = []

    # zone_info for various zones at specific times
    at_instants = [
        "2024-01-15T12:00:00Z",
        "2024-06-15T12:00:00Z",
        "2024-03-10T08:00:00Z",
        "2024-11-03T07:00:00Z",
    ]

    zone_info_zones = [
        "UTC", "America/New_York", "America/Los_Angeles", "Europe/London",
        "Europe/Paris", "Asia/Tokyo", "Asia/Singapore", "Australia/Sydney",
        "Pacific/Auckland", "America/Chicago",
    ]

    for zone in zone_info_zones:
        for at in at_instants:
            request = {"op": "zone_info", "zone": zone, "at": at}
            expected = eval_request(bin_path, request)
            cases.append(factory.make(
                category="zone_info",
                description=f"Zone info for {zone} at {at[:10]}",
                request=request,
                expected=expected,
                sources=[ZONE_INFO_SOURCE],
            ))

    # list_zones with filters
    filters = [
        None, "America/", "Europe/", "Asia/", "Pacific/",
        "America/New", "Etc/", "Australia/",
    ]

    for f in filters:
        request: Dict[str, Any] = {"op": "list_zones"}
        if f:
            request["region_filter"] = f
        expected = eval_request(bin_path, request)
        cases.append(factory.make(
            category="list_zones",
            description=f"List zones filter={f or 'none'}",
            request=request,
            expected=expected,
            sources=[ZONE_INFO_SOURCE],
        ))

    return {
        "suite": "zone_info",
        "generated_at": GENERATED_AT,
        "upstream_basis": [ZONE_INFO_SOURCE],
        "cases": cases,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate Chronow conformance corpus")
    parser.add_argument(
        "--chronow-bin",
        type=Path,
        default=DEFAULT_BIN,
        help="Path to chronow binary",
    )
    parser.add_argument(
        "--year",
        type=int,
        default=2025,
        help="Year to discover DST transitions",
    )
    args = parser.parse_args()

    bin_path = args.chronow_bin
    if not bin_path.exists():
        raise SystemExit(f"chronow binary not found at {bin_path}; run cargo build -p chronow-cli")

    CASES_DIR.mkdir(parents=True, exist_ok=True)

    iso_suite = generate_iso_suite(bin_path)
    tz_suite = generate_timezone_suite(bin_path, args.year)
    rec_suite = generate_recurrence_suite(bin_path)
    intent_suite = generate_intent_suite(bin_path)
    diff_suite = generate_diff_compare_suite(bin_path)
    snap_suite = generate_snap_suite(bin_path)
    dur_suite = generate_duration_suite(bin_path)
    interval_suite = generate_interval_suite(bin_path)
    zone_suite = generate_zone_info_suite(bin_path)

    suites = [
        ("iso_rfc3339.json", iso_suite),
        ("timezone_dst.json", tz_suite),
        ("recurrence_business.json", rec_suite),
        ("natural_language_intent.json", intent_suite),
        ("diff_compare.json", diff_suite),
        ("snap_to.json", snap_suite),
        ("duration_parsing.json", dur_suite),
        ("interval_check.json", interval_suite),
        ("zone_info.json", zone_suite),
    ]

    for filename, payload in suites:
        path = CASES_DIR / filename
        path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
        print(f"wrote {path} ({len(payload['cases'])} cases)")

    merged = merge_cases(CASES_DIR.glob("*.json"))
    metadata = {
        "generated_at": GENERATED_AT,
        "total_cases": len(merged),
        "major_zones": MAJOR_DST_ZONES,
        "available_upstream": AVAILABLE_UPSTREAMS,
        "unavailable_in_corpus": UNAVAILABLE_UPSTREAMS,
    }
    (CASES_DIR / "_metadata.json").write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    print(f"total cases: {len(merged)}")


if __name__ == "__main__":
    main()
