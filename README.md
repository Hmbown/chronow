# Chronow

[![CI](https://github.com/Hmbown/chronow/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/chronow/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/chronow)](https://pypi.org/project/chronow/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An MCP server that gives AI agents correct timezone, scheduling, and interval operations. Handles DST, business calendars, and cross-timezone conversion without guessing.

## Why

LLMs don't know what time it is. The model gets a date string injected by the system prompt -- that's it. It doesn't know the current time, it doesn't know your timezone, and it can't reliably do calendar math in its head. When a Claude tells you "the market is open" or "that's a Thursday," it's doing vibes-based arithmetic on a date string. It gets it wrong regularly.

Without a tool, the model is guessing at:
- What time it is right now (it literally doesn't know)
- What day of the week a date falls on
- Timezone conversions
- Whether DST is in effect
- Whether a date is a business day

Chronow replaces that guessing with tool calls that return correct results:

```
You: "Set up a weekly 9am standup for the next 4 weeks"

Agent calls recurrence_preview →
  Mar  2  09:00 CST  (2026-03-02T15:00:00Z)
  Mar  9  09:00 CDT  (2026-03-09T14:00:00Z)  ← DST shift, local time stays 9am
  Mar 16  09:00 CDT  (2026-03-16T14:00:00Z)
  Mar 23  09:00 CDT  (2026-03-23T14:00:00Z)
```

The UTC instant shifts by an hour at the DST boundary. The local time stays at 9:00. Without a tool, the model would likely keep the UTC constant and silently move the meeting to 10am local -- and not mention it.

## What it's for

- **Agent runtimes**: give the model `now`, `resolve_local`, `recurrence_preview`, `interval_check` etc. instead of letting it do timezone math in its head.
- **DST edges**: explicit disambiguation policies (`compatible`/`earlier`/`later`/`reject`) instead of silent one-hour bugs.
- **Business calendars**: generate recurring dates that skip weekends and holidays you specify.
- **Multi-language stacks**: 865-case conformance corpus enforces byte-identical output across Rust, TypeScript, and Python.

## What it isn't

- A calendar integration (it does not talk to Google Calendar, Outlook, etc.).
- A holiday database (you supply weekend/holiday rules via `business_calendar`).
- A fuzzy natural-language date parser (that ambiguity is exactly what Chronow avoids).

## How it works

Chronow is a Rust temporal engine exposing 15 pure-function operations. The primary interface is an **MCP server** (`chronow-mcp`) that runs over stdio -- add it to Claude Code, Claude Desktop, Cursor, or any MCP host and the agent gets correct time tools automatically.

Also ships as a **CLI** (`chronow`) and as **TypeScript/Python adapters**, all verified against the same conformance suite.

### Tools

| Tool | What it does |
|---|---|
| `now` | Current time in any timezone (auto-detects user's zone) |
| `resolve_local` | Convert a wall-clock time to UTC -- handles DST gaps/folds explicitly |
| `format_instant` | Show a UTC instant in any timezone |
| `recurrence_preview` | Generate recurring dates (daily/weekly/monthly) with optional business calendar (skip weekends + holidays) |
| `add_duration` | Add days/months/years with calendar or absolute arithmetic |
| `diff_instants` | Calendar difference between two instants (years, months, days, ...) |
| `interval_check` | Do two time ranges overlap / does one contain the other / what's the gap? |
| `snap_to` | Snap to start/end of hour, day, week, month, quarter, or year |
| `normalize_intent` | Parse a small deterministic grammar (`tomorrow at 09:00`, `next monday at 14:00`, `every weekday at 10:00`) -- rejects anything ambiguous |
| `zone_info` | Timezone offset, DST status, next transition |
| `parse_instant` | Parse ISO 8601 / RFC 3339 to UTC |
| `compare_instants` | Compare two instants (-1, 0, 1) |
| `parse_duration` / `format_duration` | ISO 8601 duration round-trip |
| `list_zones` | List IANA timezone names |

### Why not date-fns / dayjs / pendulum?

Those are fine for application code. Chronow is built for a different problem:

1. **Explicit DST disambiguation.** Four policies (`compatible`/`earlier`/`later`/`reject`) so you control what happens at gaps and folds instead of getting silently wrong results.
2. **Cross-language parity.** 865-case conformance corpus enforces byte-identical output across Rust, TypeScript, and Python.
3. **Agent-native.** Auto-detects timezone. `normalize_intent` deliberately rejects ambiguous input instead of guessing.

## Quick start

### 1. Install

**Prebuilt binaries** (recommended) -- download from [Releases](https://github.com/Hmbown/chronow/releases). Each archive includes both `chronow` (CLI) and `chronow-mcp` (MCP server) for Linux, macOS, and Windows.

**From source:**

```bash
cargo install --path mcp   # MCP server
cargo install --path cli   # CLI (optional)
```

**Docker:**

```bash
docker run -i ghcr.io/hmbown/chronow-mcp
```

**Python:** `pip install chronow` (requires `chronow` CLI on `$PATH` or `CHRONOW_BIN` set).

### 2. Add to your AI client

Replace `/path/to/chronow-mcp` with the actual path (e.g. `which chronow-mcp`).

**Claude Code:**

```bash
claude mcp add chronow /path/to/chronow-mcp
```

**Claude Desktop** -- Settings > Developer > Edit Config:

```json
{
  "mcpServers": {
    "chronow": { "command": "/path/to/chronow-mcp" }
  }
}
```

**Cursor** -- Settings > MCP, or `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "chronow": { "command": "/path/to/chronow-mcp" }
  }
}
```

### 3. Verify

Ask your AI client "What time is it?" -- it should return your local time without you specifying a timezone.

### Timezone detection

The MCP server auto-detects the user's timezone: `CHRONOW_DEFAULT_ZONE` env var > system timezone > `TZ` env var > UTC. To override:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp",
      "env": { "CHRONOW_DEFAULT_ZONE": "America/Chicago" }
    }
  }
}
```

## DST disambiguation policies

For ambiguous/non-existent local times during DST transitions:

| Policy | Ambiguous (fold) | Non-existent (gap) |
|--------|-----------------|-------------------|
| `compatible` | earlier offset | shift forward |
| `earlier` | earlier offset | shift backward |
| `later` | later offset | shift forward |
| `reject` | error | error |

## Components

| Path | Description |
|------|-------------|
| `crates/core` | Rust temporal engine (library crate + WASM cdylib) |
| `cli` | `chronow` CLI for parsing, conversion, and recurrence |
| `mcp` | `chronow-mcp` MCP server binary |
| `packages/ts` | TypeScript package (WASM + CLI adapter) |
| `packages/python` | Python package (CLI bridge) |
| `conformance/cases` | 865 canonical conformance cases (JSON) |
| `conformance/runner` | Cross-language conformance matrix runner |
| `conformance/upstream` | Upstream behavior extractions (dayjs, luxon, pendulum) |
| `spec/temporal-contract.md` | Normative behavior contract |

## Documentation

- [Getting Started](docs/getting-started.md) -- installation, first operations, MCP setup
- [Architecture](docs/architecture.md) -- design decisions and tradeoffs
- [DST Failure Modes](docs/dst-failure-modes.md) -- why temporal bugs happen
- [Migration Guide](docs/migration-guide.md) -- moving from date-fns, dayjs, luxon, pendulum
- [Benchmarks](docs/benchmarks.md) -- performance numbers
- [Agent Workflow Patterns](docs/agent-workflow-patterns.md) -- integration patterns for LLM agents
- [Upstream Behavior Discovery](spec/upstream-behavior-discovery.md) -- comparison with other libraries

## License

[MIT](LICENSE)
