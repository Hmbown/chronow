# Getting Started with Chronow

This tutorial walks you through the core operations of the `chronow` CLI: parsing timestamps, converting timezones, handling DST safely, performing duration arithmetic, and generating recurring schedules. By the end you will have a working mental model of the deterministic temporal contract that Chronow enforces across every language binding.

---

## 1. Installation

Pick whichever channel fits your stack.

### Binary (prebuilt release)

Download the latest archive for your platform from the
[Releases](https://github.com/Hmbown/chronow/releases) page, extract it, and
place the `chronow` binary on your `$PATH`:

```bash
# macOS / Linux example
tar xzf chronow-v0.2.0-x86_64-linux.tar.gz
sudo cp chronow /usr/local/bin/
chronow --help
```

### Cargo (Rust)

```bash
cargo install chronow-cli
```

### npm (TypeScript / Node.js)

```bash
npm install chronow
```

### pip (Python)

```bash
pip install chronow
```

The Python package is a thin wrapper around the `chronow` CLI, so you still need the `chronow` binary on your `$PATH` (or set `CHRONOW_BIN`).

Verify the CLI is available:

```bash
chronow --help
```

---

## 2. Parse your first timestamp

The `parse` subcommand accepts any ISO 8601 / RFC 3339 string and normalizes it
to a UTC instant with epoch values.

```bash
chronow parse --input "2024-03-10T02:30:00Z"
```

Output:

```json
{
  "ok": true,
  "value": {
    "instant": "2024-03-10T02:30:00Z",
    "epoch_seconds": 1710037800,
    "epoch_millis": 1710037800000
  }
}
```

The response always contains three fields: the canonical `instant` in UTC, the
Unix `epoch_seconds`, and `epoch_millis`. If the input carries a non-UTC offset
(e.g. `+05:30`), the instant is converted to UTC before the epoch values are
computed.

```bash
chronow parse --input "2024-03-10T08:00:00+05:30"
```

```json
{
  "ok": true,
  "value": {
    "instant": "2024-03-10T02:30:00Z",
    "epoch_seconds": 1710037800,
    "epoch_millis": 1710037800000
  }
}
```

---

## 3. Convert between timezones

Use `convert` to project a UTC instant into a target IANA timezone.

```bash
chronow convert --input "2024-03-10T02:30:00Z" --zone America/New_York
```

Output:

```json
{
  "ok": true,
  "value": {
    "instant": "2024-03-10T02:30:00Z",
    "zone": "America/New_York",
    "local": "2024-03-09T21:30:00",
    "offset_seconds": -18000,
    "zoned": "2024-03-09T21:30:00-05:00"
  }
}
```

Key fields:

| Field | Meaning |
|-------|---------|
| `local` | Wall-clock datetime in the target zone (no offset suffix). |
| `offset_seconds` | UTC offset applied, in seconds (-18000 = UTC-05:00). |
| `zoned` | Full zoned representation with offset. |

---

## 4. Handle DST safely

Daylight Saving Time creates two categories of problem times:

- **Gaps** (spring forward): a local time that never exists on the wall clock.
- **Ambiguities** (fall back): a local time that occurs twice.

Chronow never silently guesses. The `resolve` subcommand takes a
`--disambiguation` policy that makes the behavior deterministic.

### Spring-forward gap example

In `America/New_York`, clocks jump from 02:00 to 03:00 on the second Sunday of
March. The local time `2025-03-09T02:30:28` does not exist.

**`compatible` policy** (default): shift forward to the first valid local time.

```bash
chronow resolve \
  --local "2025-03-09T02:30:28" \
  --zone America/New_York \
  --disambiguation compatible
```

```json
{
  "ok": true,
  "value": {
    "input_local": "2025-03-09T02:30:28",
    "resolved_local": "2025-03-09T03:00:28",
    "instant": "2025-03-09T07:00:28Z",
    "zone": "America/New_York",
    "offset_seconds": -14400,
    "zoned": "2025-03-09T03:00:28-04:00",
    "disambiguation_applied": "shift_forward"
  }
}
```

Notice `disambiguation_applied` is `"shift_forward"` -- the engine tells you
exactly what it did.

**`reject` policy**: fail with an explicit error instead of adjusting.

```bash
chronow resolve \
  --local "2025-03-09T02:30:28" \
  --zone America/New_York \
  --disambiguation reject
```

```json
{
  "ok": false,
  "error": {
    "code": "nonexistent_local_time",
    "message": "Local time 2025-03-09T02:30:28 does not exist in America/New_York (spring-forward gap)"
  }
}
```

Available policies: `compatible`, `earlier`, `later`, `reject`. See the
[temporal contract](../spec/temporal-contract.md) for the full disambiguation
matrix.

---

## 5. Add durations

Chronow supports two arithmetic modes. The difference matters when DST
transitions or month boundaries are involved.

### Calendar arithmetic

Calendar mode applies wall-clock math in the target zone, then resolves the
resulting local time.

```bash
chronow add-duration \
  --instant "2024-01-31T12:00:00Z" \
  --duration P1M \
  --mode calendar \
  --zone America/New_York
```

```json
{
  "ok": true,
  "value": {
    "start": "2024-01-31T12:00:00Z",
    "result": "2024-02-29T12:00:00Z",
    "zone": "America/New_York",
    "arithmetic": "calendar",
    "disambiguation_applied": "exact",
    "duration": {
      "years": 0,
      "months": 1,
      "weeks": 0,
      "days": 0,
      "hours": 0,
      "minutes": 0,
      "seconds": 0
    }
  }
}
```

January 31 + 1 month = February 29 (2024 is a leap year). Month arithmetic is
day-clamped, never overflows into March.

### Absolute arithmetic

Absolute mode adds exact elapsed seconds to the instant. No timezone context is
needed.

```bash
chronow add-duration \
  --instant "2024-01-31T12:00:00Z" \
  --duration PT3600S \
  --mode absolute \
  --zone UTC
```

```json
{
  "ok": true,
  "value": {
    "start": "2024-01-31T12:00:00Z",
    "result": "2024-01-31T13:00:00Z",
    "zone": "UTC",
    "arithmetic": "absolute",
    "disambiguation_applied": "exact",
    "duration": {
      "years": 0,
      "months": 0,
      "weeks": 0,
      "days": 0,
      "hours": 1,
      "minutes": 0,
      "seconds": 0
    }
  }
}
```

Use `calendar` when you mean "same time next month". Use `absolute` when you
mean "exactly 3600 seconds from now".

---

## 6. Recurring events

The `recur` subcommand generates a bounded list of occurrences. It supports
daily, weekly, and monthly frequencies, optional weekday filters, weekend
exclusion, and explicit holiday dates.

### Business-day recurrence

Generate 5 business-day occurrences at 09:00 Eastern, skipping weekends and
Martin Luther King Jr. Day:

```bash
chronow recur \
  --start-local "2026-01-05T09:00:00" \
  --zone America/New_York \
  --freq daily \
  --count 5 \
  --exclude-weekends \
  --holidays "2026-01-19"
```

```json
{
  "ok": true,
  "value": {
    "zone": "America/New_York",
    "start_local": "2026-01-05T09:00:00",
    "rule": {
      "frequency": "daily",
      "interval": 1,
      "count": 5,
      "by_weekdays": []
    },
    "business_calendar": {
      "exclude_weekends": true,
      "holidays": ["2026-01-19"]
    },
    "occurrences": [
      {
        "local": "2026-01-05T09:00:00",
        "instant": "2026-01-05T14:00:00Z",
        "zoned": "2026-01-05T09:00:00-05:00",
        "offset_seconds": -18000,
        "resolved_local": "2026-01-05T09:00:00",
        "disambiguation_applied": "exact"
      },
      {
        "local": "2026-01-06T09:00:00",
        "instant": "2026-01-06T14:00:00Z",
        "zoned": "2026-01-06T09:00:00-05:00",
        "offset_seconds": -18000,
        "resolved_local": "2026-01-06T09:00:00",
        "disambiguation_applied": "exact"
      },
      {
        "local": "2026-01-07T09:00:00",
        "instant": "2026-01-07T14:00:00Z",
        "zoned": "2026-01-07T09:00:00-05:00",
        "offset_seconds": -18000,
        "resolved_local": "2026-01-07T09:00:00",
        "disambiguation_applied": "exact"
      },
      {
        "local": "2026-01-08T09:00:00",
        "instant": "2026-01-08T14:00:00Z",
        "zoned": "2026-01-08T09:00:00-05:00",
        "offset_seconds": -18000,
        "resolved_local": "2026-01-08T09:00:00",
        "disambiguation_applied": "exact"
      },
      {
        "local": "2026-01-09T09:00:00",
        "instant": "2026-01-09T14:00:00Z",
        "zoned": "2026-01-09T09:00:00-05:00",
        "offset_seconds": -18000,
        "resolved_local": "2026-01-09T09:00:00",
        "disambiguation_applied": "exact"
      }
    ]
  }
}
```

Note that Jan 10 and Jan 11 (Saturday/Sunday) are skipped, and the holiday on
Jan 19 would also be excluded if the count extended that far. Every occurrence
carries its own `disambiguation_applied` flag so you can audit DST behavior
per-event.

---

## 7. Use as MCP server

Chronow ships a separate MCP (Model Context Protocol) binary -- `chronow-mcp`
-- that exposes every temporal operation as a tool over the **stdio transport**
(JSON-RPC over stdin/stdout). This lets AI agents call `parse_instant`,
`resolve_local`, `add_duration`, `recurrence_preview`, and every other operation
directly without shelling out to the CLI.

Quick setup for Claude Code:

```bash
claude mcp add chronow /path/to/chronow-mcp
```

For full configuration instructions covering Claude Desktop, Cursor, and other
MCP-compatible hosts, see the **MCP server** section in the
[README](../README.md#mcp-server).

---

## 8. Next steps

- [Architecture, tradeoffs, and known gaps](./architecture.md) -- understand
  the Rust core, cross-language binding strategy, and design decisions.
- [Agent workflow patterns](./agent-workflow-patterns.md) -- common patterns
  for integrating Chronow into LLM agent pipelines.
- [Migration guide](./migration-guide.md) -- moving from `date-fns`, `dayjs`,
  `pendulum`, or raw `Date` / `datetime` to Chronow.
- [Temporal contract](../spec/temporal-contract.md) -- the normative behavior
  specification that all adapters conform to.
