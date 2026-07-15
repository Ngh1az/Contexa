# Glossary

**Project:** Contexa — AI Context Platform  
**Version:** 1.1  
**Status:** Reviewed  
**Last Updated:** 2026-07-06

---

## A

**AI Orchestrator**  
The decision engine that routes user requests to appropriate capabilities (OCR, search, memory, LLM) and manages the AI response pipeline.

**Application Enricher**  
A plugin that extracts application-specific metadata (URL, file path, git branch) from a known process. See [18_Plugin_System.md](./18_Plugin_System.md).

**Assembled Prompt**  
The final token-optimized prompt sent to an LLM, combining context, memory, search results, and user query. See [10_Prompt_Builder.md](./10_Prompt_Builder.md).

---

## C

**Capture Method**  
How visible text was obtained: `uia` (UI Automation), `ocr` (Optical Character Recognition), or `hybrid` (both).

**Confidence Score (UIA)**  
A 0.0–1.0 score indicating how complete the UIA text extraction is. Below 0.5 triggers OCR fallback.

**Context**  
Structured knowledge about the user's current desktop state: active app, window title, visible text, URL, document path, selection, and metadata.

**Context Snapshot**  
An immutable point-in-time record of context. Identified by UUID. See [06_Context_Engine.md](./06_Context_Engine.md).

**ContextEnricher**  
Rust trait implemented by plugins to add app-specific data to context snapshots.

---

## E

**Embedding**  
A fixed-dimension vector (384-dim default via fastembed; 768-dim in quality mode) representing the semantic meaning of a text chunk. Stored in sqlite-vec.

**Event Bus**  
In-process publish/subscribe system for decoupled engine communication. See [ADR/0005](../ADR/0005-event-bus-architecture.md).

**Execution Plan**  
The Orchestrator's decision output specifying which capabilities to invoke for a request.

**Exclusion List**  
User-configured apps, URLs, and window titles that Contexa will not capture.

---

## F

**Frame Differencing**  
Comparing consecutive screen captures via perceptual hashing to detect and skip unchanged regions.

---

## L

**Long-term Memory**  
Persisted memory chunks with embeddings in SQLite. Retained per user policy (default 90 days).

**LLM Provider Adapter**  
Implementation of the `LlmProvider` trait for a specific AI service (OpenAI, Ollama, etc.).

---

## M

**MCP (Model Context Protocol)**  
Open standard for AI systems to access tools and context. Contexa implements MCP server and client. See [11_MCP_Runtime.md](./11_MCP_Runtime.md).

**Memory Chunk**  
A searchable unit of stored knowledge derived from a context snapshot. Contains text, metadata, and an embedding vector.

---

## O

**Overlay**  
The floating UI window activated by `Alt + Space` for AI interactions. See [12_UI_UX.md](./12_UI_UX.md).

---

## P

**Perceptual Hash**  
A compact fingerprint of a visual frame used to detect significant changes without pixel-by-pixel comparison.

**Prompt Builder**  
Engine that assembles optimized prompts with token budgeting. See [10_Prompt_Builder.md](./10_Prompt_Builder.md).

---

## R

**Region Hashing**  
Dividing a frame into a 16×16 grid and hashing each cell to skip unchanged UI regions.

---

## S

**Semantic Search**  
Finding memory chunks by vector similarity (cosine distance) rather than keyword matching.

**Session Memory**  
Context snapshots persisted during the current login session.

**Source Ref**  
Metadata tracking which data sources (context, memory, search, timeline) contributed to an assembled prompt.

---

## T

**Timeline**  
Chronological log of user activity events: app switches, context changes, queries, and AI responses.

**Timeline Event**  
A single entry in the timeline with timestamp, summary, application, and optional duration.

---

## U

**UIA (UI Automation)**  
Windows accessibility API for traversing UI element trees and extracting text, control types, and properties.

---

## V

**Vision Engine**  
Subsystem responsible for screen capture, UIA extraction, selective OCR, and frame analysis. See [05_Vision_Engine.md](./05_Vision_Engine.md).

**VisionResult**  
Output of the Vision Engine: frame hash, UIA text, OCR text, changed regions, and capture metadata.

---

## W

**Working Memory**  
In-memory ring buffer of the last 30 minutes of context snapshots (max 200 entries).

---

## Acronyms

| Acronym | Expansion |
|---------|-----------|
| ADR | Architecture Decision Record |
| API | Application Programming Interface |
| COM | Component Object Model (Windows) |
| HWND | Window Handle (Windows) |
| IPC | Inter-Process Communication |
| LLM | Large Language Model |
| MCP | Model Context Protocol |
| NFR | Non-Functional Requirement |
| OCR | Optical Character Recognition |
| SRS | Software Requirements Specification |
| STA | Single-Threaded Apartment (COM) |
| TTL | Time To Live |
| UIA | UI Automation |
| UUID | Universally Unique Identifier |
| WAL | Write-Ahead Logging (SQLite) |

---

## References

- [01_Software_Requirements_Specification.md](./01_Software_Requirements_Specification.md) — Section 1.3 Definitions
- [README.md](./README.md)
