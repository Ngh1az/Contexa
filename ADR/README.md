# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the Contexa project.

## Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0001](./0001-rust-core-tauri-shell.md) | Rust Core with Tauri Shell | Accepted | 2026-07-06 |
| [0002](./0002-uia-first-ocr-fallback.md) | UI Automation First, OCR Fallback | Accepted | 2026-07-06 |
| [0003](./0003-sqlite-local-storage.md) | SQLite for Local-First Storage | Accepted | 2026-07-06 |
| [0004](./0004-mcp-first-integration.md) | MCP-First Ecosystem Integration | Accepted | 2026-07-06 |
| [0005](./0005-event-bus-architecture.md) | Event Bus for Engine Communication | Accepted | 2026-07-06 |
| [0006](./0006-embedding-model.md) | fastembed Default; Ollama Quality Opt-in | Accepted | 2026-07-07 |
| [0007](./0007-default-llm-strategy.md) | Default LLM Strategy (Ollama-first) | Accepted | 2026-07-06 |
| [0008](./0008-windows-com-threading.md) | Windows COM Threading Model | Accepted | 2026-07-06 |
| [0009](./0009-sqlcipher-encryption.md) | SQLCipher At-Rest Encryption (Pro v1.1) | Accepted | 2026-07-07 |
| [0010](./0010-rusqlite-database-access.md) | rusqlite Database Access Layer | Accepted | 2026-07-07 |
| [0011](./0011-duckduckgo-default-search.md) | DuckDuckGo as Default Search Provider | Accepted | 2026-07-07 |
| [0012](./0012-local-reranking.md) | Local Reranking Model for Semantic Search | Proposed | 2026-07-14 |
| [0013](./0013-tree-sitter-fallback-parsing.md) | Tree-sitter Fallback for Code Structure | Proposed | 2026-07-14 |
| [0014](./0014-local-semantic-web-search.md) | Local Semantic Ranking for Web Search (No Cloud AI Search API) | Proposed | 2026-07-15 |

## Creating New ADRs

1. Copy template from any existing ADR
2. Number sequentially (next: 0015)
3. Set status to "Proposed"
4. Submit as PR for team review
5. Update status to "Accepted" after approval

## References

- [Contexa Documentation](../docs/README.md)
- [Michael Nygard's ADR Template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
