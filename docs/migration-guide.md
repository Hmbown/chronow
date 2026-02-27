# Migration Guide

## date-fns -> Chronow
- `parseISO(x)` -> `{"op":"parse_instant","input":x}`
- `formatISO(date, opts)` -> `{"op":"format_instant","instant":...,"zone":...,"format":...}`
- `addBusinessDays` / `eachDayOfInterval` style workflows -> `recurrence_preview` with `business_calendar`.

## dayjs / luxon -> Chronow
- Replace implicit local-zone assumptions with explicit `zone` on every request.
- Replace plugin-based ambiguity handling with explicit `disambiguation`.
- Move natural-language input through `normalize_intent` deterministic grammar.

## pendulum -> Chronow
- `pendulum.parse` / `.in_timezone` -> `parse_instant` + `format_instant`.
- Interval recurrence with business logic -> `recurrence_preview`.

## Practical migration steps
1. Wrap old entry points with Chronow request builders.
2. Run old/new outputs against a staging fixture.
3. Gate rollout with `conformance/runner/run.py --strict` in CI.
4. Promote only when parity + DST boundary tests pass.
