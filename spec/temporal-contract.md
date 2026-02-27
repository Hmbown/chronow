# Chronow Temporal Contract

## Purpose
Chronow defines a deterministic, agent-safe temporal contract across Rust, TypeScript, and Python.

## Conformance-first model
- Canonical cases live in `conformance/cases/*.json`.
- Every adapter must return byte-equivalent JSON (after canonical key ordering) for every case.
- `conformance/runner/run.py --strict` is the source of truth.

## Request envelope
Chronow uses tagged JSON operations:
- `parse_instant`
- `format_instant`
- `resolve_local`
- `add_duration`
- `recurrence_preview`
- `normalize_intent`
- `diff_instants`
- `compare_instants`
- `snap_to`
- `parse_duration`
- `format_duration`
- `interval_check`
- `zone_info`
- `list_zones`
- `now`

Each response is:
```json
{
  "ok": true,
  "value": {"...": "..."}
}
```
or
```json
{
  "ok": false,
  "error": {"code": "...", "message": "..."}
}
```

## Deterministic DST conflict policy
`disambiguation` values:
- `compatible`: ambiguous -> earlier offset, nonexistent -> shift forward to first valid local time.
- `earlier`: ambiguous -> earlier offset, nonexistent -> shift backward to last valid local time.
- `later`: ambiguous -> later offset, nonexistent -> shift forward to first valid local time.
- `reject`: error on ambiguous or nonexistent local time.

No silent fuzzy behavior is allowed.

## Arithmetic semantics
- `absolute`: adds exact elapsed seconds to an instant.
- `calendar`: applies wall-clock calendar math in local zone, then resolves with `disambiguation`.
- Month arithmetic is day-clamped (e.g. Jan 31 + 1 month -> Feb 28/29).

## Recurrence semantics
- Supported frequencies: `daily`, `weekly`, `monthly`.
- Optional `by_weekdays` filter.
- Business calendar supports weekend exclusion and explicit holiday dates.
- Recurrence generation is deterministic and bounded by requested `count`.

## Natural-language intent grammar
Deterministic grammar only:
- `tomorrow at HH:MM [in Zone]`
- `next <weekday> at HH:MM [in Zone]`
- `in N days at HH:MM [in Zone]`
- `on YYYY-MM-DD at HH:MM [in Zone]`
- `every weekday at HH:MM [in Zone] [for N occurrences]`
- `every <weekday> at HH:MM [in Zone] [for N occurrences]`

Any other input must return `unsupported_intent`.

## Diff semantics
- `diff_instants`: computes the calendar difference between two instants in a given zone.
- Uses greedy month subtraction: add months to `start` until it would exceed `end`, then count remaining days/hours/minutes/seconds.
- End-of-month clamping: Jan 31 + 1 month = Feb 28/29 (so Jan 31 → Feb 28 = 1 month, 0 days).
- Direction is `end - start`. If `end < start`, all components are negative.
- Returns `{years, months, days, hours, minutes, seconds, total_seconds}`.

## Compare semantics
- `compare_instants`: parses two instants to UTC and returns `-1`, `0`, or `1`.

## Snap semantics
- `snap_to`: snaps an instant to the boundary of a calendar unit in a given zone.
- Units: `hour`, `day`, `week`, `month`, `quarter`, `year`.
- Edges: `start` or `end`.
- Hour snap: `start` snaps to :00:00 of the current hour; `end` snaps to :59:59 of the current hour.
- Day/week/month/quarter/year snap: `start` snaps to 00:00:00; `end` snaps to 23:59:59 of the last day of the unit.
- Week snap uses `week_starts_on` (default: `monday`). Supported values: `monday`, `sunday`.
- DST gaps at the snap boundary are resolved with `compatible` disambiguation.
- Quarter boundaries: Q1=Jan, Q2=Apr, Q3=Jul, Q4=Oct.

## Duration parsing and formatting (ISO 8601)
- `parse_duration`: parses an ISO 8601 duration string `PnYnMnWnDTnHnMnS`.
- `M` before `T` = months. `M` after `T` = minutes. `P1MT1M` = 1 month + 1 minute.
- Weeks are normalized to days (1W = 7D) in the parsed output.
- Empty duration `P` or `PT` with no components is an error.
- Negative durations: a leading `-` before `P` (e.g., `-P1DT2H`) negates all components.
  All parsed fields will be negative. This is consistent with ISO 8601-2:2019 sign convention.
- `format_duration`: converts a `DurationSpec` struct to ISO 8601 string.
- Weeks are folded into days. All components emitted (zero components included as `0`).
- Output format: `PnYnMnDTnHnMnS`.
- Negative output: if any field is negative, the formatted string is prefixed with `-`
  and all component magnitudes are emitted as absolute values (e.g., `-P1Y2M3DT4H5M6S`).
- `add_duration` works naturally with negative durations — negative fields subtract time.

## Interval semantics
- `interval_check`: compares two time intervals.
- Intervals are normalized: if `start > end`, they are swapped.
- Modes:
  - `overlap`: strict — `max(starts) < min(ends)`. Touching endpoints are NOT overlapping.
  - `contains`: `a.start <= b.start && b.end <= a.end`.
  - `gap`: returns gap interval and `gap_seconds` when no overlap; `null` gap when overlapping.
- Result shape: `{result: bool, gap: {start, end} | null, gap_seconds: number | null}`.

## Zone info semantics
- `zone_info`: returns offset, DST flag, abbreviation, and next transition for a zone at a given instant.
- `at` defaults to current time if omitted.
- DST detection: compares current offset to January 1 offset. If different and current offset is larger, it's DST.
- Next transition: 6-hour step search up to 1 year, then binary search to 1-second precision.
- Fixed-offset zones (e.g., `Etc/UTC`) have no transitions → `next_transition: null`.
- `list_zones`: returns all IANA zone names, optionally filtered by region prefix, sorted alphabetically.

## Non-deterministic operations
- `now`: returns the current time. Excluded from conformance cases because output is inherently non-deterministic.
- `zone_info` without `at` is also non-deterministic; conformance cases always supply `at`.

## Upstream behavior extraction
Derived from Conform upstream corpus:
- `date-fns` parse behavior (`parseISO` tests)
- `date-fns` format behavior (`formatISO` tests)
- `date-fns` business-day and interval behavior (`addBusinessDays`, `eachDayOfInterval` tests)
- `date-fns` diff behavior (`differenceInMonths`, `differenceInDays`, `intervalToDuration`, `compareAsc`)
- `date-fns` snap behavior (`startOfDay`, `endOfDay`, `startOfWeek`, `endOfMonth`, `startOfQuarter`, etc.)
- `date-fns` duration format behavior (`formatISODuration`)
- `date-fns` interval overlap behavior (`areIntervalsOverlapping`, `isWithinInterval`)

Unavailable in current Conform corpus snapshot:
- `dayjs`, `luxon`, `pendulum`, `arrow`, `dateparser`, `chrono`, `jiff`, `nodatime`

These are documented and tracked in `conformance/cases/_metadata.json`.
