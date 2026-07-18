# SP-06: MCP Server + Cursor Integration

**Date:** 2026-07-18
**Owner:** —
**Status:** Pass

## Summary

A minimal `rmcp` 0.8 stdio server exposing a stub `get_current_context` tool passes MCP-protocol
verification via the official MCP Inspector CLI (`@modelcontextprotocol/inspector`) and via a
purpose-built Rust client using `rmcp`'s own `TokioChildProcess` transport — the same launch
pattern Cursor and Claude Desktop use for stdio MCP servers. All four SP-06 pass criteria are met.
Real Cursor IDE (`C:\Program Files\cursor`) is installed on this machine, but by owner decision
(2026-07-18) the spike server was **not** added to the user's live `~/.cursor/mcp.json` — protocol
compliance via Inspector CLI + a compliant client library was judged sufficient evidence, to avoid
touching the user's real IDE configuration for a throwaway spike server.

## Method

1. Implemented `ContexaSpikeServer` (`src/main.rs`) with `#[tool_router]`/`#[tool_handler]` macros,
   one tool `get_current_context(max_chars: Option<u32>) -> String` returning stub JSON context
   (app, window title, visible text, timestamp) — mirrors the shape ADR-0004 specifies for the real
   tool, without wiring the actual `contexa-context`/`contexa-vision` pipeline (out of scope for a
   protocol spike).
2. Verified schema + tool listing + tool call via `npx @modelcontextprotocol/inspector --cli`
   (`tools/list`, `tools/call`) — this is the same client library MCP-compatible tools (including
   Cursor) use to speak the protocol.
3. Built a second binary (`src/client.rs`) using `rmcp`'s `TokioChildProcess` client transport —
   spawns the server as a child process over stdio (the exact mechanism Cursor uses per its
   `mcp.json` `command`/`args` config), calls `list_tools` then `call_tool` 50 times on a
   **persistent connection**, and measures round-trip latency in-process (excludes Node/npx
   cold-start, which is irrelevant to the protocol's own latency).

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Server recognized (tool listing + valid JSON schema) | ✅ | `tools/list` returns `get_current_context` with valid JSON Schema (`inspectorCLI` output) | ✅ |
| Tool call succeeds | ✅ | `tools/call` returns well-formed JSON content, `isError: false` | ✅ |
| Latency (tool call round-trip, persistent connection) | < 10 ms | p50=0.16 ms, p95=0.24 ms, p99=0.30 ms (50 calls) | ✅ |
| JSON schema valid | ✅ | Inspector CLI parses `inputSchema` (draft-07) without error; `max_chars` optional `integer` | ✅ |

## Observations

1. **npx/Node cold-start (~3.8s wall time) is not tool-call latency.** Each Inspector CLI invocation
   spawns a fresh Node process; measuring wall-clock around that gives a meaningless ~3.8s figure.
   The spec's `< 10ms` target is about the protocol round-trip on an already-running connection,
   which the dedicated Rust client (`sp06_client`) measures correctly: p95 = 0.24 ms.
2. **`rmcp` 0.8.5's API differs from the `rust-sdk` reference examples** (which target a newer/older
   version): `CallToolRequestParams::new(...).with_arguments(...)` doesn't exist in 0.8.5 — use the
   plain struct `CallToolRequestParam { name, arguments }` instead. Worth flagging for whoever wires
   `crates/contexa-mcp` for real: pin and verify the exact `rmcp` version against its actual API,
   not just the workspace `rmcp = "1"` line (crates.io only has 0.8.x as of this spike — the
   workspace pin is aspirational/needs revisiting when `contexa-mcp` is implemented).
3. Cursor IDE is installed on this machine (`~/.cursor/mcp.json` already configures one real MCP
   server, `codegraph`). A true Cursor-recognizes-server end-to-end check was deliberately skipped —
   owner chose not to touch the live IDE config for a throwaway spike binary. Inspector CLI + a
   spec-compliant client library exercising the same child-process/stdio transport Cursor uses is
   accepted as equivalent evidence for this gate.

## Recommendation

**Proceed with MCP-first integration (ADR-0004 validated).** Protocol mechanics (stdio transport,
tool schema, tool call, latency) all check out well within target. When `crates/contexa-mcp` is
implemented for real: (a) confirm the `rmcp` version pin against crates.io's actual releases, (b)
wire `get_current_context` to the real `contexa-context` pipeline instead of the stub, (c) do one
manual Cursor smoke test at that point (real tool, real config) rather than for this throwaway spike
binary.

## Raw Data

- Server binary: `target/release/sp06_mcp_cursor.exe`
- Client binary: `target/release/sp06_client.exe`
- Commands:
  - `npx -y @modelcontextprotocol/inspector --cli target/release/sp06_mcp_cursor.exe --method tools/list`
  - `npx -y @modelcontextprotocol/inspector --cli target/release/sp06_mcp_cursor.exe --method tools/call --tool-name get_current_context --tool-arg max_chars=200`
  - `./target/release/sp06_client.exe`
- Client output:
  ```
  Tools recognized: ["get_current_context"]
  Tool call latency (in-process round-trip, 50 calls): p50=0.16ms p95=0.24ms p99=0.30ms
  ```
