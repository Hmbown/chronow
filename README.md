# Chronow

[![CI](https://github.com/Hmbown/chronow/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/chronow/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/chronow)](https://pypi.org/project/chronow/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Deterministic temporal primitives for agents**: DST-safe timezone operations, recurrence generation, interval checks, and a strict cross-language conformance suite.

## Quick start

From the repo:

```bash
# Current time in a zone
cargo run -p chronow-cli -- now --zone America/Chicago

# Resolve a local time to a UTC instant (DST-safe, explicit disambiguation)
cargo run -p chronow-cli -- resolve --local 2025-03-09T02:30:00 --zone America/New_York --disambiguation compatible

# Run the cross-language conformance matrix (Rust/TS/Python)
python3 conformance/runner/run.py --matrix rust ts python --strict
```

## The problem

LLMs guess at time. DST, timezones, and business-day rules turn those guesses into real bugs.

Chronow gives agents deterministic time primitives (plus a test corpus) so scheduling, conversion, and "does this overlap?" become tool calls, not hallucinations.

## Where it helps

Chronow is useful when you care about correctness and reproducibility more than "best effort" parsing:

- Agent runtimes: give the model tools like `now`, `resolve_local`, and `diff_instants` instead of letting it guess.
- DST edges: pick an explicit policy for gaps/folds (`compatible`/`earlier`/`later`/`reject`) instead of getting silent one-hour bugs.
- Multi-language stacks: CI enforces byte-identical JSON output across Rust/TypeScript/Python against an 865-case corpus.
- Deterministic intent normalization: `normalize_intent` accepts a small, explicit grammar (24-hour `HH:MM`) and returns `unsupported_intent` for everything else.

## What it isn't

- A calendar integration (it does not talk to Google Calendar, Outlook, etc.).
- A holiday database (you supply weekend/holiday rules via `business_calendar`).
- A fuzzy natural-language date parser (that ambiguity is exactly what Chronow avoids).

## What it does

Chronow is a Rust temporal engine exposing 15 pure-function operations through:

- **MCP server (`chronow-mcp`)** -- stdio-based MCP server for Claude Code/Desktop, Cursor, or any MCP host.
- **CLI (`chronow`)** -- parse/convert/resolve/diff/recur from the terminal (JSON in, JSON out).
- **Adapters** -- TypeScript (WASM/CLI) and Python (CLI bridge), verified against the conformance suite.

### Key capabilities

| What you need | Tool | Example |
|---|---|---|
| Current time in any timezone | `now` | "What time is it in Tokyo?" |
| Is today a trading day? | `recurrence_preview` | Business calendar (weekends + holiday list you supply) |
| When does this option expire? | `add_duration`, `snap_to` | Add duration, snap to end of day/month |
| Schedule a recurring meeting | `recurrence_preview` | Weekly MWF at 9am, auto-adjusts across DST |
| Convert between timezones | `resolve_local` + `format_instant` | "3pm Chicago time in London" |
| Days until a deadline | `diff_instants` | Calendar diff with timezone awareness |
| Do these meetings overlap? | `interval_check` | Overlap/containment/gap detection |
| What day of the week is March 14? | `format_instant` | Deterministic, no guessing |
| Next business day after a holiday | `recurrence_preview` | Skip weekends + custom holiday list |
| Normalize a deterministic phrase | `normalize_intent` | `next monday at 09:00 in America/New_York` |

### Why not just use date-fns / dayjs / pendulum?

You can, and they're great for application code. Chronow is different in three ways:

1. **Explicit DST disambiguation.** Four policies (`compatible`, `earlier`, `later`, `reject`) give deterministic control over gap and fold resolution. No silent "wrong by one hour" bugs.
2. **Cross-language parity.** Rust is the reference engine; TypeScript (WASM/CLI) and Python (CLI bridge) are checked for byte-identical output against an 865-case conformance corpus. If your agent pipeline spans languages, results won't drift.
3. **Agent-native.** The MCP server auto-detects the user's timezone. `normalize_intent` is deliberately strict: it normalizes a small grammar and rejects everything else instead of guessing.

## Install

### Prebuilt binaries (recommended)

Download from [Releases](https://github.com/Hmbown/chronow/releases) -- each archive includes both the `chronow` CLI and the `chronow-mcp` server for Linux, macOS, and Windows.

### From source (CLI + MCP)

```bash
cargo build --release -p chronow-cli -p chronow-mcp

# Or install into ~/.cargo/bin
cargo install --path cli
cargo install --path mcp
```

### Python (PyPI)

```bash
pip install chronow
```

The Python package shells out to the `chronow` CLI. Install the CLI (from a GitHub release or from source) and ensure `chronow` is on your `$PATH` (or set `CHRONOW_BIN`).

### Rust / TypeScript bindings

Rust (`crates/core`) and TypeScript (`packages/ts`) bindings live in this repo and are exercised by the conformance suite. They are not published to crates.io/npm yet.

## MCP server

Chronow ships an MCP (Model Context Protocol) server binary -- `chronow-mcp` -- that
exposes every temporal operation as a tool. It uses the **stdio transport**
(JSON-RPC over stdin/stdout), so the host process launches it directly; no
network ports or URLs are needed.

### Timezone auto-detection

The MCP server automatically detects the user's timezone at startup using this fallback chain:

1. **`CHRONOW_DEFAULT_ZONE` env var** -- explicit override, highest priority
2. **System timezone** -- detected from the OS (via `iana_time_zone`)
3. **`TZ` env var** -- standard Unix timezone variable
4. **UTC** -- final fallback

The detected timezone is used as the default for all tools that accept a `zone` parameter. This means agents don't need to ask users for their timezone -- it just works.

To override the detected timezone, set the env var in your MCP config:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp",
      "env": {
        "CHRONOW_DEFAULT_ZONE": "America/Chicago"
      }
    }
  }
}
```

### Available tools

| Tool | Description |
|------|-------------|
| `now` | Current time (defaults to user's timezone) |
| `parse_instant` | Parse ISO 8601 / RFC 3339 string to UTC instant |
| `format_instant` | Format a UTC instant in a given timezone |
| `resolve_local` | Resolve a local datetime to UTC (DST-aware) |
| `add_duration` | Add a duration using absolute or calendar arithmetic |
| `recurrence_preview` | Generate recurring datetime occurrences |
| `normalize_intent` | Normalize a deterministic intent grammar into a structured request |
| `diff_instants` | Calendar difference between two instants |
| `compare_instants` | Compare two instants (-1, 0, 1) |
| `snap_to` | Snap an instant to the edge of a calendar unit |
| `parse_duration` | Parse an ISO 8601 duration string |
| `format_duration` | Format duration components to ISO 8601 |
| `interval_check` | Check two intervals for overlap / containment / gap |
| `zone_info` | Get timezone offset, DST status, abbreviation |
| `list_zones` | List IANA timezone names |

`normalize_intent` is intentionally strict (for determinism). It accepts patterns like `tomorrow at 15:30`, `next monday at 09:00`, `on 2026-03-01 at 09:00`, and returns `unsupported_intent` for everything else.

### Install the MCP server

**From source (requires Rust toolchain)**

```bash
cargo build --release -p chronow-mcp
cp target/release/chronow-mcp /usr/local/bin/
```

Or install directly:

```bash
cargo install --path mcp
```

**From prebuilt binaries**

Download the latest release for your platform from the
[Releases](https://github.com/Hmbown/chronow/releases) page, extract the
archive, and place `chronow-mcp` somewhere on your `$PATH`.

**Docker**

```bash
docker run -i ghcr.io/hmbown/chronow-mcp
```

If `docker pull` is denied, use the GitHub release binaries instead (container publishing may lag behind tags).

### Configure your AI client

> Replace `/path/to/chronow-mcp` below with the actual absolute path to the
> binary (e.g. the result of `which chronow-mcp` after installing).

#### Claude Code

```bash
claude mcp add chronow /path/to/chronow-mcp
```

Or add to `.mcp.json` / `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp"
    }
  }
}
```

To set a specific timezone:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp",
      "env": {
        "CHRONOW_DEFAULT_ZONE": "America/Chicago"
      }
    }
  }
}
```

#### Claude Desktop

Open **Settings > Developer > Edit Config** and add to
`claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp"
    }
  }
}
```

Restart Claude Desktop after saving.

#### Cursor

Open **Settings > MCP** and add a new global server, or edit
`.cursor/mcp.json` in the project root:

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp"
    }
  }
}
```

### Verify it works

Ask your AI client to call the `now` tool -- it should return the current time in your local timezone without you specifying it:

```
What time is it right now?
```

Expected result (timezone auto-detected; shape is the engine response):

```json
{
  "ok": true,
  "value": {
    "epoch_seconds": 1718472600,
    "instant": "2024-06-15T17:30:00Z",
    "local": "2024-06-15T12:30:00",
    "offset_seconds": -18000,
    "zone": "America/Chicago",
    "zoned": "2024-06-15T12:30:00-05:00"
  }
}
```

Note: if you test `chronow-mcp` by hand over stdin/stdout, you must send a `notifications/initialized` notification after the `initialize` request (MCP clients do this automatically).

## Deterministic conflict policy

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
