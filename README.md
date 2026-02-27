# Chronow

Chronow is an agent-native, deterministic temporal engine with a conformance-first workflow.

## Components

- `crates/core`: Rust temporal engine.
- `cli`: `chronow` CLI for parsing, conversion, and recurrence previews.
- `packages/ts`: TypeScript package (WASM-ready + CLI adapter).
- `packages/python`: Python binding/package.
- `conformance/cases`: Canonical conformance corpus JSON.
- `conformance/runner`: Cross-language conformance matrix runner.
- `spec/temporal-contract.md`: Normative behavior contract.

## Deterministic conflict policy

For ambiguous/non-existent local times:
- `compatible`: ambiguous -> earlier, nonexistent -> shift forward
- `earlier`: ambiguous -> earlier, nonexistent -> shift backward
- `later`: ambiguous -> later, nonexistent -> shift forward
- `reject`: fail with explicit error

## MCP server

Chronow ships an MCP (Model Context Protocol) server binary -- `chronow-mcp` -- that
exposes every temporal operation as a tool. It uses the **stdio transport**
(JSON-RPC over stdin/stdout), so the host process launches it directly; no
network ports or URLs are needed.

### Available tools

| Tool | Description |
|------|-------------|
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
| `now` | Current time (optionally projected into a timezone) |

### Install

**From source (requires Rust toolchain)**

```bash
# Build the MCP binary from the workspace root
cargo build --release -p chronow-mcp

# The binary is at target/release/chronow-mcp
# Copy it somewhere on your PATH if you like:
cp target/release/chronow-mcp /usr/local/bin/
```

Or install it directly with `cargo install`:

```bash
cargo install --path mcp
```

**From prebuilt binaries**

Download the latest release for your platform from the
[Releases](https://github.com/punkpeye/chronow/releases) page, extract the
archive, and place `chronow-mcp` somewhere on your `$PATH`.

### Configure your AI client

The MCP server uses the **stdio transport** -- the client launches the binary
and communicates over stdin/stdout. No flags, environment variables, or config
files are required.

> Replace `/path/to/chronow-mcp` below with the actual absolute path to the
> binary (e.g. the result of `which chronow-mcp` after installing).

#### Claude Code

Add the server via the CLI:

```bash
claude mcp add chronow /path/to/chronow-mcp
```

Or add it manually to your MCP config (project `.mcp.json` or
`~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "chronow": {
      "command": "/path/to/chronow-mcp"
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

After configuring your client, ask it to call the `parse_instant` tool:

```
Parse this timestamp: 2024-06-15T12:00:00Z
```

You should see a tool call to `parse_instant` with the following result:

```json
{
  "ok": true,
  "op": "parse_instant",
  "instant": "2024-06-15T12:00:00Z",
  "epoch_seconds": 1718452800,
  "epoch_millis": 1718452800000
}
```

You can also test the binary directly from a terminal. The MCP server reads
JSON-RPC messages from stdin, so you can send an `initialize` handshake
followed by a `tools/call`:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | chronow-mcp
```

If the binary is installed correctly, it will print a JSON-RPC response
containing the server info and capabilities.

### Transport note

`chronow-mcp` uses the **stdio** transport exclusively. It reads newline-
delimited JSON-RPC messages from **stdin** and writes responses to **stdout**.
No HTTP server is started, no ports are opened, and no authentication is
required. This makes it safe to run in sandboxed environments and trivial to
configure in any MCP-compatible host.

## Quick start

```bash
cargo run -p chronow-cli -- parse --input 2024-04-10T00:00:00Z
cargo run -p chronow-cli -- convert --input 2024-04-10T00:00:00Z --zone America/New_York
cargo run -p chronow-cli -- recur --start-local 2026-03-01T09:00:00 --zone America/New_York --freq daily --count 5
```

Run full conformance matrix:

```bash
python3 conformance/runner/run.py --matrix rust ts python
```

Regenerate corpus:

```bash
python3 scripts/generate_conformance_cases.py --chronow-bin target/debug/chronow
```
