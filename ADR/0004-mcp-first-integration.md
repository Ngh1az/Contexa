# ADR-0004: MCP-First Ecosystem Integration

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team

---

## Context

Contexa's long-term vision is to become the Context Layer for all AI — not another AI assistant. External AI tools (Cursor, Claude Desktop, custom agents) need access to desktop context, timeline, and memory.

Integration options:
- **Custom REST API** — Proprietary HTTP endpoints
- **gRPC** — High-performance RPC framework
- **Model Context Protocol (MCP)** — Emerging standard for AI tool/context integration
- **Language-specific SDKs** — Python, TypeScript, Rust client libraries

## Decision

Adopt **Model Context Protocol (MCP)** as the primary integration interface. Implement Contexa as an **MCP Server** exposing context tools, and optionally as an **MCP Client** for external tool access.

## Rationale

| Factor | MCP | Custom REST | gRPC |
|--------|-----|-------------|------|
| AI ecosystem adoption | Growing rapidly | None | Limited |
| Cursor support | Native | Custom integration | Custom integration |
| Claude Desktop support | Native | Custom integration | Custom integration |
| Standardization | Open protocol | Proprietary | Proprietary |
| Maintenance | Community-driven spec | Internal only | Internal only |
| Transport options | stdio, HTTP/SSE | HTTP | HTTP/2 |

MCP is becoming the standard protocol for AI tool integration. Cursor and Claude Desktop already support MCP servers. By exposing Contexa as an MCP server, any MCP-compatible AI client can immediately access desktop context without custom integration work.

## Consequences

**Positive:**
- Immediate compatibility with Cursor, Claude Desktop, and other MCP clients
- Standard tool schema (JSON Schema) for context APIs
- Positions Contexa as infrastructure, not a competing assistant
- Community can build integrations without Contexa team involvement

**Negative:**
- MCP specification is still evolving; breaking changes possible
- Limited to MCP's tool/resource model (no streaming context in v1)
- Requires user to generate and configure auth tokens
- Additional complexity of MCP server implementation

## Exposed Tools

| Tool | Purpose |
|------|---------|
| `get_current_context` | Current desktop context |
| `get_visible_text` | Active window text |
| `get_recent_context` | Recent context history |
| `get_timeline` | Chronological activity |
| `search_context` | Semantic memory search |

## References

- [11_MCP_Runtime.md](../docs/11_MCP_Runtime.md)
- [03_API_Interface_Specification.md](../docs/03_API_Interface_Specification.md)
- [Model Context Protocol](https://modelcontextprotocol.io/)
