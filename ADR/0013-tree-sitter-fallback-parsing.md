# ADR-0013: Tree-sitter Fallback for Code Structure Extraction

**Status:** Proposed  
**Date:** 2026-07-14  
**Deciders:** Architecture Team  
**Target:** v1.2+ (evaluation)

---

## Context

IDE Deep Integration ([27_IDE_LSP_Integration.md](../docs/27_IDE_LSP_Integration.md)) gets code structure (symbols, diagnostics) from a **Contexa extension inside the IDE**, pushed over local IPC. The core deliberately runs no LSP server.

This leaves a gap: editors without a Contexa extension (Notepad++, Sublime, Vim/Neovim without setup, JetBrains before v1.2) yield only flat UIA text — no symbols, no structure. Tree-sitter could close the gap: the core parses the **file the user is editing** (path known from UIA title/enricher) and extracts symbols locally.

This is an architecture change, not an optimization: today the core never reads source files from disk; all code context arrives via UIA or is pushed by the extension.

Options considered:

- **Tree-sitter in core** — bundled grammars; parse active file on demand
- **Extension-only (status quo)** — structure only where a Contexa extension exists
- **LLM-based extraction** — ask the configured LLM to extract symbols from visible text

## Decision

**Proposed, gated evaluation for v1.2** — implement only if v1.1 telemetry/feedback shows meaningful usage from non-extension editors. If implemented:

1. **Scope:** parse only the file whose path is already in `ContextSnapshot` (from UIA or enricher) and only when no IDE extension payload is present for that snapshot. Never scan directories or workspaces.
2. **Privacy:** file reads pass through the existing exclusion filter ([16_Security_Privacy.md](../docs/16_Security_Privacy.md)); excluded paths (e.g. `**/secrets/**`) are never opened. Parsed symbols go to `ContextSnapshot.metadata` under the same truncation rules as UIA text.
3. **Grammars:** bundle a fixed top set (Rust, TypeScript/JS, Python, Go, Java, C/C++, C#) — no dynamic grammar loading in v1.
4. **Budget:** runs as a normal enricher under the plugin sandbox rules ([18_Plugin_System.md](../docs/18_Plugin_System.md) §8.1) — 20 ms timeout; on timeout, snapshot proceeds without symbols. Incremental re-parse (tree-sitter's strength) keeps steady-state cost near zero.

## Rationale

| Factor | Tree-sitter | Extension-only | LLM extraction |
|--------|------------|----------------|----------------|
| Coverage (any editor) | Yes | No | Yes |
| Accuracy of symbols | High (grammar-exact) | Highest (LSP) | Low/variable |
| Latency | ~1–10 ms incremental | 0 (pushed) | ≥ 1 s |
| Privacy | Local file read (new surface) | No file read | Text to LLM |
| Binary size | +2–5 MB (7 grammars) | 0 | 0 |
| Diagnostics / references | No | Yes | No |

Tree-sitter complements — never replaces — the extension path: LSP data (diagnostics, references, git) is richer and stays authoritative when available.

## Consequences

**Positive:**
- Code structure for every editor, not just VS Code/Cursor
- Deterministic, offline, fast; fits the enricher model without new threads

**Negative:**
- Core gains a new capability class: reading user files from disk. Must be a separate settings toggle (default **off** until reviewed) and documented in the privacy policy ([25_Privacy_Policy_Draft.md](../docs/25_Privacy_Policy_Draft.md))
- +2–5 MB binary; grammar version maintenance
- File-on-disk may lag unsaved editor state — symbols can be stale; acceptable for context purposes, must be flagged in metadata (`ide.symbols_source = "tree-sitter/disk"`)

## Exit Criteria for the Evaluation Spike (2 days, time-boxed)

1. Parse + symbol extraction < 20 ms p95 on a 5K-line Rust and TypeScript file
2. Exclusion filter verified: excluded path is never opened (test with file audit)
3. Binary size delta measured with 7 bundled grammars

## References

- [27_IDE_LSP_Integration.md](../docs/27_IDE_LSP_Integration.md)
- [18_Plugin_System.md](../docs/18_Plugin_System.md)
- [16_Security_Privacy.md](../docs/16_Security_Privacy.md)
- [28_Tech_Expansion_Plan.md](../docs/28_Tech_Expansion_Plan.md)
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
