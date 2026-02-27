# DST Failure Modes

## Non-existent local times (spring forward)
Example: `2023-03-12T02:00:00 America/New_York` does not exist.

Chronow behavior:
- `compatible` / `later`: shifts to first valid time after gap.
- `earlier`: shifts to last valid time before gap.
- `reject`: returns `non_existent_local_time`.

## Ambiguous local times (fall back)
Example: `2023-11-05T01:30:00 America/New_York` occurs twice.

Chronow behavior:
- `compatible` / `earlier`: choose earlier offset instance.
- `later`: choose later offset instance.
- `reject`: returns `ambiguous_local_time`.

## Calendar vs absolute arithmetic drift
- `absolute` preserves elapsed seconds.
- `calendar` preserves wall-clock intent in the target zone.
Across DST boundaries these can differ by one hour.

## Common integration mistakes
- Storing local wall time without zone.
- Reconstructing recurrences from UTC-only timestamps.
- Mixing parser defaults across libraries without explicit disambiguation.

## Operational guidance
- Persist UTC instant + IANA zone + local display.
- Use corpus-backed tests for every zone-specific workflow.
- Prefer `reject` in finance/compliance paths.
