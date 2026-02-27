# Upstream Behavior Discovery

Source corpus: `/Volumes/VIXinSSD/conform/conform/upstreams`

## Available temporal upstream in corpus
- `date-fns/date-fns` (TypeScript)

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

## Requested upstreams not present in current Conform snapshot
- `dayjs`
- `luxon`
- `pendulum`
- `arrow`
- `dateparser`
- `chrono`
- `jiff`
- `nodatime`

These are tracked in `conformance/cases/_metadata.json` under `unavailable_in_corpus`.
