# Contexa Documentation



**AI Context Platform** — The Context Infrastructure for AI.



**Documentation Version:** 1.3  

**Status:** Reviewed — architecture decisions locked  

**Last Updated:** 2026-07-07



---



## About



Contexa is an AI Context & Memory Platform that continuously builds structured context from desktop activity, enabling any AI system to understand what the user is working on. It is **not** a chatbot, OCR tool, or AI copilot — it is the **context layer** beneath AI.



**Tagline:** AI Context Platform  

**Vision:** Become the Context Infrastructure for AI.  

**Mission:** Provide every AI with real-time desktop context and memory.



---



## Documentation Index



### Core



| # | Document | Description |

|---|----------|-------------|

| 00 | [Project Vision](./00_Project_Vision.md) | Vision, mission, competitive moat, success metrics |

| 01 | [Software Requirements Specification](./01_Software_Requirements_Specification.md) | Functional and non-functional requirements |

| 02 | [System Architecture](./02_System_Architecture.md) | Architecture, crate deps, COM threading |

| 03 | [API & Interface Specification](./03_API_Interface_Specification.md) | Tauri IPC, MCP tools, engine traits |

| 04 | [Database Design](./04_Database_Design.md) | SQLite schema, rusqlite, vectors, SQLCipher |



### Engines



| # | Document | Description |

|---|----------|-------------|

| 05 | [Vision Engine](./05_Vision_Engine.md) | Capture, UIA, OCR, Windows API mapping |

| 06 | [Context Engine](./06_Context_Engine.md) | Context assembly, enrichment, caching |

| 07 | [Memory Engine](./07_Memory_Engine.md) | Timeline, hierarchical memory, entity linking |

| 08 | [AI Orchestrator](./08_AI_Orchestrator.md) | Request routing, capability decisions |

| 09 | [Search Engine](./09_Search_Engine.md) | DuckDuckGo default, provider adapters |

| 10 | [Prompt Builder](./10_Prompt_Builder.md) | Token-aware prompt assembly |

| 11 | [MCP Runtime](./11_MCP_Runtime.md) | MCP tools, Resources (v1.1), client |



### Product & UX



| # | Document | Description |

|---|----------|-------------|

| 12 | [UI / UX Design](./12_UI_UX.md) | Overlay, component specs, interaction matrix |

| 21 | [Competitive Analysis](./21_Competitive_Analysis.md) | Landscape, positioning, SWOT |

| 24 | [Business Model](./24_Business_Model.md) | Pricing, GTM, metrics |

| 25 | [Privacy Policy (Draft)](./25_Privacy_Policy_Draft.md) | Privacy policy for legal review |



### Engineering



| # | Document | Description |

|---|----------|-------------|

| 13 | [Test Plan](./13_Test_Plan.md) | Tests, traceability matrix, benchmarks |

| 14 | [Development Roadmap](./14_Development_Roadmap.md) | Phased plan incl. spike gate (Phase 0.5) |

| 15 | [Risk Analysis](./15_Risk_Analysis.md) | Risk register with mitigations |

| 16 | [Security & Privacy](./16_Security_Privacy.md) | Threat model, GDPR/CCPA |

| 17 | [Performance Optimization](./17_Performance_Optimization.md) | Targets, baseline protocol |

| 18 | [Plugin System](./18_Plugin_System.md) | Context enricher plugins |

| 19 | [Coding Standards](./19_Coding_Standards.md) | Rust, TypeScript, Git conventions |

| 20 | [Deployment](./20_Deployment.md) | Build, signing, distribution |

| 27 | [IDE LSP Integration](./27_IDE_LSP_Integration.md) | VS Code extension, LSP IPC, MCP |

| 28 | [Tech Expansion Plan](./28_Tech_Expansion_Plan.md) | Post-GA tech dispositions, gates, conditional items |

| 29 | [Dev Environment Setup](./29_Dev_Environment_Setup.md) | Prerequisites, bootstrap, run, CI-equivalent checks |



### Reference



| # | Document | Description |

|---|----------|-------------|

| 22 | [Technical Spike Plan](./22_Technical_Spike_Plan.md) | Pre-build validation (SP-01–SP-09) |

| 23 | [Glossary](./23_Glossary.md) | Terms and acronyms |

| 26 | [Reference Repositories](./26_Reference_Repos.md) | GitHub repos mapped to engines |



---



## Architecture Decision Records



See [ADR/](../ADR/) for architectural decisions.



| ADR | Title | Status |

|-----|-------|--------|

| [0001](../ADR/0001-rust-core-tauri-shell.md) | Rust Core with Tauri Shell | Accepted |

| [0002](../ADR/0002-uia-first-ocr-fallback.md) | UI Automation First, OCR Fallback | Accepted |

| [0003](../ADR/0003-sqlite-local-storage.md) | SQLite for Local-First Storage | Accepted |

| [0004](../ADR/0004-mcp-first-integration.md) | MCP-First Ecosystem Integration | Accepted |

| [0005](../ADR/0005-event-bus-architecture.md) | Event Bus for Engine Communication | Accepted |

| [0006](../ADR/0006-embedding-model.md) | fastembed Default; Ollama Quality Opt-in | Accepted |

| [0007](../ADR/0007-default-llm-strategy.md) | Default LLM Strategy (Ollama-first) | Accepted |

| [0008](../ADR/0008-windows-com-threading.md) | Windows COM Threading Model | Accepted |

| [0009](../ADR/0009-sqlcipher-encryption.md) | SQLCipher At-Rest Encryption (Pro v1.1) | Accepted |

| [0010](../ADR/0010-rusqlite-database-access.md) | rusqlite Database Access Layer | Accepted |

| [0011](../ADR/0011-duckduckgo-default-search.md) | DuckDuckGo Default Search Provider | Accepted |

| [0012](../ADR/0012-local-reranking.md) | Local Reranking Model for Semantic Search | Proposed |

| [0013](../ADR/0013-tree-sitter-fallback-parsing.md) | Tree-sitter Fallback for Code Structure | Proposed |



---



## Post-GA v1.1 Priority Features



| Feature | Doc | Priority | Target |

|---------|-----|----------|--------|

| Hierarchical Memory | [07_Memory_Engine.md](./07_Memory_Engine.md) §13 | P1 | v1.1 |

| Entity Linking | [07_Memory_Engine.md](./07_Memory_Engine.md) §14 | P2 | v1.1 |

| MCP Resources | [11_MCP_Runtime.md](./11_MCP_Runtime.md) §13 | P1 | v1.1 |

| IDE LSP Integration | [27_IDE_LSP_Integration.md](./27_IDE_LSP_Integration.md) | P1 | v1.1 |

| SQLCipher (Pro) | [04_Database_Design.md](./04_Database_Design.md) §16 | P2 | v1.1 (gated SP-09) |



See [14_Development_Roadmap.md](./14_Development_Roadmap.md) §10 for timeline.



---



## Document Status



| Version | Date | Status |

|---------|------|--------|

| 1.0 | 2026-07-06 | Draft — Initial architecture docs |

| 1.1 | 2026-07-06 | Reviewed — Spikes, ADRs, competitive analysis |

| 1.2 | 2026-07-07 | Reviewed — v1.1 priority features specified |

| 1.3 | 2026-07-07 | Reviewed — Consistency audit; ADR-0010/0011; locked stack |



---



## Reading Order



| Role | Start Here |

|------|-----------|

| **Product / PM** | 00 → 21 → 24 → 14 |

| **Architect** | 02 → ADR/ → 04 → 22 → 26 · **AGENTS.md** |

| **Rust Engineer** | 02 → 26 → 05 → 06 → 07 → 19 → 22 |

| **Frontend Engineer** | 03 → 12 → 19 |

| **QA** | 01 → 13 → 22 |

| **Security** | 16 → 25 → 15 |



---



## Tech Stack (Locked v1.3)



| Layer | Technology |

|-------|------------|

| Desktop Shell | Tauri 2.x |

| UI | React 18, TypeScript, TailwindCSS |

| Core | Rust (edition 2021) |

| Database | SQLite 3 + sqlite-vec via **rusqlite** + **refinery** |

| Embeddings (default) | fastembed / all-MiniLM-L6-v2 (384-dim) |

| Embeddings (quality) | nomic-embed-text via Ollama (768-dim, opt-in) |

| Default LLM | Ollama (llama3.2:3b) — user-configurable |

| Default Search | DuckDuckGo (when search enabled) |

| Protocol | Model Context Protocol (MCP) via rmcp |



---



## Design Principles



- **Context First** — Every decision optimizes context quality

- **Privacy by Design** — Local-first; user controls all data

- **Low Resource Usage** — UIA-first; adaptive scheduling

- **AI Agnostic** — Support all major LLM providers

- **MCP Native** — Context layer for the AI ecosystem



---



## License



Copyright © 2026 Contexa. All rights reserved.

