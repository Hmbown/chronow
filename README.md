# Chronow

[![CI](https://github.com/Hmbown/chronow/actions/workflows/ci.yml/badge.svg)](https://github.com/Hmbown/chronow/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/chronow-core)](https://crates.io/crates/chronow-core)
[![npm](https://img.shields.io/npm/v/chronow)](https://www.npmjs.com/package/chronow)
[![PyPI](https://img.shields.io/pypi/v/chronow)](https://pypi.org/project/chronow/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Deterministic temporal primitives for AI agents** -- timezone-safe datetime operations that give agents the same reliable building blocks humans take for granted: "Is today a trading day?", "When does this option expire?", "Schedule this for the next business day."

## The problem

LLMs are bad at time. Ask an AI what day of the week March 14, 2025 falls on and it might guess wrong. Ask it whether US markets are open and it has no reliable way to check. Ask it to schedule something "next Tuesday at 3pm CST" and it may silently pick the wrong UTC instant because of DST.

This isn't a model intelligence problem -- it's a tooling problem. Agents need deterministic temporal primitives the same way they need calculators for math. Chronow provides those primitives.

## What it does

Chronow is a Rust temporal engine exposing 15 pure-function operations across three surfaces:

- **MCP server** -- plug into Claude Code, Claude Desktop, Cursor, or any MCP host. The agent calls tools like `now`, `diff_instants`, `recurrence_preview` instead of guessing.
- **Library** -- Rust crate, TypeScript/WASM package, Python package. Same engine, byte-identical results across all three.
- **CLI** -- parse, convert, diff, and recur from the terminal.

### Key capabilities

| What you need | Tool | Example |
|---|---|---|
| Current time in any timezone | `now` | "What time is it in Tokyo?" |
| Is today a trading day? | `recurrence_preview` | Business calendar with weekends + holidays excluded |
| When does this option expire? | `add_duration`, `snap_to` | Add duration, snap to end of day/month |
| Schedule a recurring meeting | `recurrence_preview` | Weekly MWF at 9am, auto-adjusts across DST |
| Convert between timezones | `resolve_local` + `format_instant` | "3pm Chicago time in London" |
| Days until a deadline | `diff_instants` | Calendar diff with timezone awareness |
| Do these meetings overlap? | `interval_check` | Overlap/containment/gap detection |
| What day of the week is March 14? | `format_instant` | Deterministic, no guessing |
| Next business day after a holiday | `recurrence_preview` | Skip weekends + custom holiday list |

### Why not just use date-fns / dayjs / pendulum?

You can, and they're great for application code. Chronow is different in three ways:

1. **Explicit DST disambiguation.** Four policies (`compatible`, `earlier`, `later`, `reject`) give deterministic control over gap and fold resolution. No silent "wrong by one hour" bugs.
2. **Cross-language parity.** The same Rust engine powers TypeScript (WASM) and Python. All three produce byte-identical results, verified by 865 conformance cases. If your agent pipeline spans languages, results won't drift.
3. **Agent-native.** The MCP server auto-detects the user's timezone, so agents don't need to ask. Every operation returns structured JSON that agents can reason over directly.

## Use cases

### Financial & trading
- **Trading day validation.** "Is today a market day?" -- use `recurrence_preview` with `exclude_weekends: true` and a holiday list for NYSE/NASDAQ/etc. No more arguing with an AI about whether a particular Monday is indeed a trading day.
- **Option expiration.** Third Friday of the month, snap to market close (4pm ET) -- `recurrence_preview` + `snap_to`.
- **Settlement dates.** T+1 / T+2 calculations that correctly skip weekends and holidays.
- **Market hours across timezones.** "Are Tokyo markets open right now?" -- `now` + `zone_info` + `interval_check`.

### Agentic scheduling
- **Calendar coordination.** Schedule across timezones without DST surprises. A weekly 9am standup doesn't silently shift an hour when clocks change.
- **Deadline tracking.** "How many business days until the filing deadline?" -- deterministic answer, not a guess.
- **Recurring tasks.** Generate the next N occurrences of a cron-like schedule, aware of DST transitions and business calendars.

### The bigger picture
These tools are temporal primitives. The hope is that operations like "is this a business day" and "what's the next occurrence of this schedule" become native capabilities in agent runtimes. Until then, the MCP server bridges the gap -- giving any MCP-compatible agent access to deterministic temporal reasoning today.

## Architecture

```
                    +------------------+
                    |   chronow-core   |   Rust temporal engine
                    |  (crates/core)   |   15 pure-function ops
                    +--------+---------+
                             |
              +--------------+--------------+
              |              |              |
     +--------v---+   +-----v------+  +----v-------+
     | chronow CLI|   |chronow-mcp |  | WASM export|
     | (cli/)     |   | (mcp/)     |  | (cdylib)   |
     +-----+------+   +-----+------+  +-----+------+
           |               |               |
     +-----v------+  stdio transport  +----v-------+
     | Python pkg |        |          |  TS package |
     | (CLI bridge|  +-----v------+   | (WASM/CLI) |
     +------------+  | AI agents  |   +------------+
                     +------------+
```

**Conformance suite:** `conformance/cases/` (865 canonical JSON cases) + `conformance/runner/` (cross-language matrix runner verifying Rust, TypeScript, and Python parity).

## Install

**Cargo (Rust library)**
```bash
cargo add chronow-core
```

**npm (TypeScript/Node.js)**
```bash
npm install chronow
```

**pip (Python)**
```bash
pip install chronow
```

**Prebuilt binaries**

Download from [Releases](https://github.com/Hmbown/chronow/releases) -- includes `chronow` CLI and `chronow-mcp` server for Linux, macOS, and Windows.

**Docker (MCP server)**
```bash
docker run -i ghcr.io/hmbown/chronow-mcp
```

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
| `normalize_intent` | Parse natural-language temporal intent |
| `diff_instants` | Calendar difference between two instants |
| `compare_instants` | Compare two instants (-1, 0, 1) |
| `snap_to` | Snap an instant to the edge of a calendar unit |
| `parse_duration` | Parse an ISO 8601 duration string |
| `format_duration` | Format duration components to ISO 8601 |
| `interval_check` | Check two intervals for overlap / containment / gap |
| `zone_info` | Get timezone offset, DST status, abbreviation |
| `list_zones` | List IANA timezone names |

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

Expected result (timezone auto-detected):

```json
{
  "ok": true,
  "op": "now",
  "instant": "2024-06-15T17:30:00Z",
  "local": "2024-06-15T12:30:00",
  "zoned": "2024-06-15T12:30:00-05:00",
  "zone": "America/Chicago"
}
```

Or test the binary directly:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | chronow-mcp
```

## Feature matrix

| Operation | Rust | TypeScript | Python | MCP | CLI |
|-----------|:----:|:----------:|:------:|:---:|:---:|
| parse_instant | x | x | x | x | x |
| format_instant | x | x | x | x | x |
| resolve_local | x | x | x | x | x |
| add_duration | x | x | x | x | x |
| recurrence_preview | x | x | x | x | x |
| normalize_intent | x | x | x | x | x |
| diff_instants | x | x | x | x | x |
| compare_instants | x | x | x | x | x |
| snap_to | x | x | x | x | x |
| parse_duration | x | x | x | x | x |
| format_duration | x | x | x | x | x |
| interval_check | x | x | x | x | x |
| zone_info | x | x | x | x | x |
| list_zones | x | x | x | x | x |
| now | x | x | x | x | x |

## Deterministic conflict policy

For ambiguous/non-existent local times during DST transitions:

| Policy | Ambiguous (fold) | Non-existent (gap) |
|--------|-----------------|-------------------|
| `compatible` | earlier offset | shift forward |
| `earlier` | earlier offset | shift backward |
| `later` | later offset | shift forward |
| `reject` | error | error |

## Quick start

```bash
# Parse a timestamp
cargo run -p chronow-cli -- parse --input 2024-04-10T00:00:00Z

# Convert to a timezone
cargo run -p chronow-cli -- convert --input 2024-04-10T00:00:00Z --zone America/New_York

# Generate recurring occurrences
cargo run -p chronow-cli -- recur --start-local 2026-03-01T09:00:00 --zone America/New_York --freq daily --count 5
```

Run the full conformance matrix:

```bash
python3 conformance/runner/run.py --matrix rust ts python
```

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
