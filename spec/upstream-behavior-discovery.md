# Upstream Behavior Discovery

Source corpus: `conformance/upstream/`

## Available upstream extractions

### date-fns (TypeScript)

Original source: `conform/upstreams/typescript/date-fns__date-fns`

Behavior extracted from tests:
- `src/parseISO/test.ts`:
  - ISO parse validity/invalidity boundaries
  - offset parsing (`Z`, `+hh:mm`, `+hhmm`, `+hh`)
  - DST-gap compatibility behavior in `America/New_York`
- `src/formatISO/test.ts`:
  - extended/basic/date/time output shapes
  - zero-offset handling
- `src/addBusinessDays/test.ts`:
  - weekend skipping and forward/backward business-day semantics
- `src/eachDayOfInterval/test.ts`:
  - interval inclusivity, directionality, and step handling

### dayjs (TypeScript)

Extracted via `scripts/extract_upstream_behavior.js`. Output: `conformance/upstream/dayjs.json`

Scenarios: ISO parsing, timezone formatting, DST resolution, duration addition, diff, duration parsing.

**Where chronow disagrees with dayjs and why:**

| Area | dayjs behavior | chronow behavior | Rationale |
|------|---------------|-------------------|-----------|
| Date-only parsing | `2024-09-17` parses relative to local system timezone | Parses as UTC midnight (`2024-09-17T00:00:00Z`) | Determinism: system timezone must not affect parse results |
| DST +1d (calendar) | `+1d` across spring-forward = absolute 24h (`06:00Z→06:00Z`) | Calendar: same wall-clock time next day (`06:00Z→05:00Z`) | Calendar arithmetic preserves local time; dayjs conflates calendar and absolute |
| Negative duration | `-P1D` parsed but sign is lost (`total_seconds: 86400`) | `-P1D` correctly produces negative components | ISO 8601 requires preserving sign on negative durations |
| Month diff (Jan 31 → Feb 29) | 0 months | 1 month | chronow uses calendar month boundaries; dayjs uses strict 28/30/31 day thresholds |
| Disambiguation | No disambiguation parameter; implicit resolution | Explicit `compatible`/`earlier`/`later`/`reject` policies | Agents need control over ambiguous time resolution |

### luxon (TypeScript)

Extracted via `scripts/extract_upstream_behavior.js`. Output: `conformance/upstream/luxon.json`

Scenarios: ISO parsing, timezone formatting, DST resolution, duration addition, diff, duration parsing.

**Where chronow disagrees with luxon and why:**

| Area | luxon behavior | chronow behavior | Rationale |
|------|---------------|-------------------|-----------|
| DST +1d (calendar) | Calendar arithmetic: same wall-clock (`05:00Z`) | Same as luxon | Agreement |
| DST fall-back +1d | Calendar arithmetic: wall-clock preserved (`06:00Z`) | Same as luxon | Agreement |
| Negative duration | `-P1D` → `P-1D` (component-level negation) | `-P1D` (leading sign) | chronow follows ISO 8601 leading-sign convention |
| Month diff (Jan 31 → Feb 29) | 0 months (29 days) | 1 month | chronow treats Feb 29 as end-of-month, counting a full calendar month from Jan 31 |
| Disambiguation | Implicit: gaps shift forward, folds pick first | Explicit 4-mode policy | Agents need deterministic control |

### pendulum (Python)

Extracted via `scripts/extract_upstream_pendulum.py`. Output: `conformance/upstream/pendulum.json`

Scenarios: ISO parsing, timezone formatting, DST resolution, duration addition, diff, duration parsing.

**Where chronow disagrees with pendulum and why:**

| Area | pendulum behavior | chronow behavior | Rationale |
|------|------------------|-------------------|-----------|
| DST +1d | Calendar arithmetic (`05:00Z`) | Same as pendulum | Agreement |
| Month diff (Jan 31 → Feb 29) | 1 month | Same as pendulum | Agreement on calendar month semantics |
| Duration parsing | No ISO 8601 duration string parsing | Full ISO 8601 duration parsing (`P1Y2M3DT4H5M6S`, `-P1D`) | Agents exchange durations as ISO strings; parsing is essential |
| Disambiguation | Implicit: gaps shift forward, folds pick first | Explicit 4-mode policy | Agents need deterministic control |

## Behavioral summary across upstreams

| Scenario | chronow | date-fns | dayjs | luxon | pendulum |
|----------|---------|----------|-------|-------|----------|
| DST gap resolution | configurable (4 policies) | shift forward | shift forward | shift forward | shift forward |
| DST fold resolution | configurable (4 policies) | earlier | later | later (first) | later (first) |
| Calendar +1d across DST | wall-clock preserved | wall-clock preserved | absolute 24h | wall-clock preserved | wall-clock preserved |
| Jan 31 + 1 month | Feb 28/29 (clamp) | Feb 29 (clamp) | Feb 29 (clamp) | Feb 29 (clamp) | Feb 29 (clamp) |
| Month diff Jan 31→Feb 29 | 1 month | - | 0 months | 0 months | 1 month |
| Negative duration parse | supported (`-P1D`) | - | lossy (sign dropped) | supported (`P-1D`) | unsupported |
| Date-only parse | UTC midnight | UTC midnight | local timezone | UTC midnight | UTC midnight |
| ISO 8601 duration parse | full support | partial | full | full | unsupported |

## Requested upstreams not yet extracted
- `arrow` (Python)
- `dateparser` (Python)
- `chrono` (Rust)
- `jiff` (Rust)
- `nodatime` (C#)

These remain tracked in `conformance/cases/_metadata.json` under `unavailable_in_corpus`.
