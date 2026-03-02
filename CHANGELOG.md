# Changelog

All notable changes to the **Chronow** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Version sync requirement** -- the following files MUST carry the same
> version string at all times:
>
> - `Cargo.toml` workspace version (`workspace.package.version`)
> - `packages/ts/package.json` (`version`)
> - `packages/python/pyproject.toml` (`project.version`)
>
> CI enforces this invariant. Bump all three atomically when cutting a release.

## [Unreleased]

## [0.2.1] - 2026-03-01

### Changed

- Rewrote README to lead with the actual problem: LLMs don't know what time it is and guess wrong. Added concrete DST recurrence example.
- Reorganized README structure: Why > What it's for > How it works > Quick start. Consolidated duplicate tool tables, compressed MCP client setup instructions.
- Sharpened tool descriptions to say what each one does in plain language.

### Infrastructure

- Added PyPI trusted publishing to release workflow.
- Updated Dockerfile base image to `rust:1-alpine` (unpinned minor).

## [0.2.0] - 2025-06-15

### Added

#### Temporal engine -- 15 operations

| # | Operation | Description |
|---|-----------|-------------|
| 1 | `parse_instant` | Parse ISO 8601 / RFC 3339 string into a UTC instant with epoch timestamps |
| 2 | `format_instant` | Format a UTC instant in a given IANA timezone (extended, basic, date, time) |
| 3 | `resolve_local` | Resolve a wall-clock local time to a UTC instant with DST disambiguation |
| 4 | `add_duration` | Add an ISO 8601 duration to an instant (absolute or calendar arithmetic) |
| 5 | `recurrence_preview` | Generate recurring occurrences with RRULE-style rules and business calendar |
| 6 | `normalize_intent` | Parse natural-language scheduling phrases into structured requests |
| 7 | `diff_instants` | Compute calendar difference between two instants in a timezone |
| 8 | `compare_instants` | Compare two instants and return their ordering (-1, 0, 1) |
| 9 | `snap_to` | Snap (floor/ceil) an instant to a calendar-unit boundary (day, week, month, quarter, year) |
| 10 | `parse_duration` | Parse an ISO 8601 duration string into year/month/week/day/hour/minute/second components |
| 11 | `format_duration` | Format duration components into an ISO 8601 duration string |
| 12 | `interval_check` | Test overlap, containment, or gap between two time intervals |
| 13 | `zone_info` | Return timezone metadata: UTC offset, DST status, abbreviation, next transition |
| 14 | `list_zones` | List all known IANA timezone names with optional region-prefix filter |
| 15 | `now` | Return the current UTC instant, optionally projected into a timezone |

#### CLI -- 18 subcommands

- `parse` -- parse an ISO 8601 / RFC 3339 datetime string
- `convert` -- format a UTC instant into a target timezone
- `resolve` -- resolve a local datetime to UTC with disambiguation
- `add-duration` -- add an ISO 8601 duration to an instant
- `recur` -- generate recurring datetime occurrences
- `normalize-intent` -- convert natural language into a structured request
- `diff` -- compute the calendar difference between two instants
- `compare` -- compare two instants
- `snap` -- snap an instant to a calendar-unit boundary
- `parse-duration` -- parse an ISO 8601 duration string
- `format-duration` -- format duration components into ISO 8601
- `interval-check` -- test interval overlap, containment, or gap
- `zone-info` -- get timezone metadata at a given instant
- `list-zones` -- list IANA timezone names
- `now` -- get the current time
- `eval` -- evaluate an arbitrary JSON request against the engine
- `eval-corpus` -- batch-evaluate a conformance corpus file
- `completions` -- generate shell completion scripts (bash, zsh, fish, powershell)

#### MCP server -- 15 tools

Stdio-based Model Context Protocol server (`chronow-mcp`) exposing all 15
engine operations as MCP tools with JSON Schema parameter validation and
structured error propagation (`is_error: true` for engine failures).

#### Language bindings

- **TypeScript** (`packages/ts`) -- dual-mode bindings: WASM for in-process
  calls, CLI bridge for Node.js subprocess fallback. Full type declarations.
- **Python** (`packages/python`) -- CLI bridge bindings wrapping the `chronow`
  binary. Synchronous API surface.

#### Conformance

- 865-case conformance corpus across 10 category files covering ISO/RFC 3339
  parsing, timezone/DST handling, duration parsing, interval checks,
  natural-language intent, recurrence with business calendars, and snap-to
  operations.
- Cross-language parity enforcement: Rust, TypeScript, and Python must produce
  byte-identical JSON output for every corpus case.

### Infrastructure

- GitHub Actions CI workflow (lint, test, conformance parity checks)
- Release workflow with multi-platform binary builds
- Coverage reporting workflow
