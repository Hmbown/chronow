# Multi-stage build for chronow-mcp
# Produces a minimal image (~20MB) that runs the MCP server on stdio.
#
# Build:  docker build -t chronow-mcp .
# Run:    docker run -i chronow-mcp

# ── Stage 1: Build ───────────────────────────────────────────────────────
FROM rust:1.84-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY cli/ cli/
COPY mcp/ mcp/

RUN cargo build --release -p chronow-mcp && \
    strip target/release/chronow-mcp

# ── Stage 2: Runtime ─────────────────────────────────────────────────────
FROM alpine:3.21

COPY --from=builder /src/target/release/chronow-mcp /usr/local/bin/chronow-mcp

ENTRYPOINT ["chronow-mcp"]
