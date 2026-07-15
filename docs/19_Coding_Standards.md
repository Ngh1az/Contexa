# Coding Standards

**Project:** Contexa — AI Context Platform  
**Version:** 1.3  
**Status:** Reviewed  
**Last Updated:** 2026-07-07

---

## 1. Overview

This document defines coding conventions, tooling, and best practices for the Contexa codebase. Consistency across Rust core, TypeScript UI, and documentation ensures maintainability as the team and codebase grow.

---

## 2. Goals

1. Consistent code style across all languages and crates
2. Automated enforcement via linters and formatters in CI
3. Clear patterns for error handling, logging, and testing
4. Documentation standards for public APIs
5. Review-ready code on every pull request

---

## 3. Repository Structure

```
contexa/
├── apps/
│   ├── desktop/                 # Tauri app (Rust + React)
│   │   ├── src-tauri/           # Rust backend
│   │   └── src/                 # React frontend
│   └── web/                     # Next.js marketing site
├── crates/
│   ├── contexa-core/            # Shared types, traits, event bus
│   ├── contexa-vision/          # Vision Engine
│   ├── contexa-context/         # Context Engine
│   ├── contexa-memory/          # Memory Engine
│   ├── contexa-orchestrator/    # AI Orchestrator
│   ├── contexa-search/          # Search Engine
│   ├── contexa-prompt/          # Prompt Builder
│   ├── contexa-mcp/             # MCP Runtime
│   ├── contexa-llm/             # LLM adapters
│   └── contexa-db/              # Database layer
├── docs/                        # Architecture documentation
├── ADR/                         # Architecture Decision Records
├── Cargo.toml                   # Workspace root
├── package.json                 # Node workspace root
├── turbo.json                   # Turborepo config
└── .github/workflows/           # CI/CD
```

---

## 4. Rust Standards

### 4.1 Edition & Toolchain

- **Edition:** 2021
- **MSRV:** 1.75.0 (minimum supported Rust version)
- **Toolchain:** Stable channel; pin in `rust-toolchain.toml`

### 4.2 Formatting

- **Tool:** `rustfmt` with default settings
- **Enforcement:** CI check via `cargo fmt --check`

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
```

### 4.3 Linting

- **Tool:** `clippy` with warnings as errors in CI
- **Enforcement:** `cargo clippy -- -D warnings`

```toml
# Cargo.toml (workspace)
[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
unwrap_used = "warn"
expect_used = "warn"
```

### 4.4 Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Crates | `snake_case` with `contexa-` prefix | `contexa-vision` |
| Modules | `snake_case` | `frame_differencer` |
| Types / Traits | `PascalCase` | `ContextSnapshot`, `VisionEngine` |
| Functions | `snake_case` | `get_current_context` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_CHUNK_SIZE` |
| Type parameters | Single uppercase or descriptive | `T`, `Engine` |

### 4.5 Error Handling

Use `thiserror` for library errors; `anyhow` only in application binaries.

```rust
// crates/contexa-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ContexaError {
    #[error("Capture failed: {reason}")]
    CaptureFailed { reason: String },

    #[error("Context unavailable")]
    ContextUnavailable,

    #[error("LLM provider error ({provider}): {message}")]
    LlmProviderError { provider: String, message: String },
}

pub type Result<T> = std::result::Result<T, ContexaError>;
```

**Rules:**
- Never use `.unwrap()` in library code
- Use `.expect("reason")` only for logically impossible states
- Propagate errors with `?` operator
- Add context with `.map_err()` when crossing crate boundaries

### 4.6 Logging & Tracing

```rust
use tracing::{debug, info, warn, error, instrument};

#[instrument(skip(self), fields(hwnd = %hwnd))]
pub fn extract_uia_text(&self, hwnd: isize) -> Result<UiaResult> {
    debug!("Starting UIA extraction");
    let result = self.do_extract(hwnd)?;
    debug!(text_len = result.text.len(), "UIA extraction complete");
    Ok(result)
}
```

**Rules:**
- Use `tracing` crate (not `log`)
- `error!` for failures requiring attention
- `warn!` for degraded operation (fallback used)
- `info!` for significant lifecycle events
- `debug!` for development diagnostics
- Never log sensitive data (API keys, passwords, full context text)

### 4.7 Documentation

```rust
/// Extracts text content from the UI Automation tree of the given window.
///
/// # Arguments
/// * `hwnd` - Window handle of the target window
///
/// # Returns
/// * `Ok(UiaResult)` - Extracted text with confidence score
/// * `Err(ContexaError::CaptureFailed)` - UIA tree walk failed
///
/// # Performance
/// Typical extraction completes in < 100ms.
pub fn extract_uia_text(&self, hwnd: isize) -> Result<UiaResult> {
```

**Rules:**
- All public items must have doc comments
- Include examples for traits and complex functions
- Run `cargo doc --no-deps` in CI to catch broken docs

### 4.8 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_detector_detects_app_switch() {
        let detector = ChangeDetector::new();
        let snap1 = make_snapshot("chrome.exe", "Google");
        let snap2 = make_snapshot("Code.exe", "main.rs");
        assert!(detector.has_changed(&snap2));
    }

    #[tokio::test]
    async fn memory_search_returns_relevant_results() {
        let engine = setup_test_memory_engine().await;
        engine.ingest(&make_snapshot_with_text("OAuth 2.0 flow")).await.unwrap();
        let results = engine.search("OAuth", SearchOptions::default()).await.unwrap();
        assert!(!results.is_empty());
    }
}
```

**Rules:**
- Unit tests in `#[cfg(test)]` module within same file
- Integration tests in `crates/*/tests/` directory
- Use `mockall` for trait mocks
- Use in-memory SQLite for database tests
- Name tests: `function_name_condition_expected_behavior`

---

## 5. TypeScript / React Standards

### 5.1 Tooling

| Tool | Purpose |
|------|---------|
| TypeScript 5.x | Type safety |
| ESLint | Linting |
| Prettier | Formatting |
| Vitest | Unit testing |
| Playwright | E2E testing |

### 5.2 Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Components | `PascalCase` | `OverlayWindow.tsx` |
| Hooks | `camelCase` with `use` prefix | `useContextUpdate` |
| Utilities | `camelCase` | `formatTimestamp` |
| Types / Interfaces | `PascalCase` | `ContextSnapshot` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RESPONSE_LENGTH` |
| Files (components) | `PascalCase.tsx` | `ChatPanel.tsx` |
| Files (utilities) | `camelCase.ts` | `tauriCommands.ts` |

### 5.3 Component Structure

```tsx
// src/components/ChatPanel.tsx
import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { HandleRequestParams, AiChunk } from '../types';

interface ChatPanelProps {
  onResponseComplete: (requestId: string) => void;
}

export function ChatPanel({ onResponseComplete }: ChatPanelProps) {
  const [query, setQuery] = useState('');
  const [response, setResponse] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = useCallback(async () => {
    if (!query.trim()) return;
    setIsLoading(true);
    setResponse('');

    const { request_id } = await invoke<{ request_id: string }>('handle_request', {
      action: 'chat',
      query,
      stream: true,
    } satisfies HandleRequestParams);

    const unlisten = await listen<AiChunk>('ai-chunk', (event) => {
      if (event.payload.request_id !== request_id) return;
      setResponse((prev) => prev + event.payload.content);
      if (event.payload.done) {
        setIsLoading(false);
        onResponseComplete(request_id);
        unlisten();
      }
    });
  }, [query, onResponseComplete]);

  return (
    // JSX
  );
}
```

### 5.4 TypeScript Rules

- Strict mode enabled (`"strict": true`)
- No `any` type; use `unknown` and narrow
- Prefer `interface` over `type` for object shapes
- Use `satisfies` for type checking without widening
- Shared types between Rust and TS generated or manually synced

### 5.5 Styling

- **TailwindCSS** utility classes only
- No custom CSS files unless absolutely necessary
- Use design tokens from [12_UI_UX.md](./12_UI_UX.md)
- Dark theme as default

---

## 6. Git Conventions

### 6.1 Branch Naming

```
feature/vision-engine-frame-diff
fix/context-cache-race-condition
docs/api-specification-update
chore/ci-pipeline-setup
```

### 6.2 Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(vision): add frame differencing with perceptual hash
fix(context): prevent race condition in cache update
docs(architecture): add MCP runtime specification
test(memory): add semantic search integration tests
chore(deps): update tauri to 2.1.0
perf(vision): downscale frames for hash computation
refactor(orchestrator): extract decision engine
```

### 6.3 Pull Request Guidelines

- One feature/fix per PR
- Include test coverage for new code
- Update documentation if API changes
- Link to relevant SRS requirement or ADR
- Require 1 approval before merge
- CI must pass (lint, test, build)

---

## 7. CI/CD Checks

```yaml
# Required checks on every PR
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test --workspace
- cargo doc --no-deps
- pnpm lint
- pnpm typecheck
- pnpm test
- cargo deny check licenses
```

---

## 8. Dependency Management

### 8.1 Rust

- Pin direct dependencies in crate `Cargo.toml`
- Use `cargo-deny` for license and advisory checking
- Prefer well-maintained crates with > 1M downloads
- Minimize dependency count; audit new additions

### 8.2 Node

- Use `pnpm` with lockfile
- Pin major versions in `package.json`
- Run `npm audit` in CI
- Prefer packages with active maintenance

---

## 9. Architecture Patterns

### 9.1 Engine Pattern

Each engine is a separate crate implementing a trait defined in `contexa-core`:

```
contexa-core (traits, types, event bus)
    ↑
contexa-vision (implements VisionEngine)
contexa-context (implements ContextEngine)
contexa-memory (implements MemoryEngine)
    ↑
contexa-orchestrator (coordinates engines)
    ↑
apps/desktop (wires everything together)
```

### 9.2 Event Bus Pattern

Engines communicate via events, not direct calls (except through Orchestrator):

```rust
// Publish
event_bus.publish(ContexaEvent::ContextUpdate(snapshot));

// Subscribe
let mut rx = event_bus.subscribe();
while let Ok(event) = rx.recv().await {
    match event {
        ContexaEvent::ContextUpdate(snap) => memory_engine.ingest(&snap).await?,
        _ => {}
    }
}
```

### 9.3 Adapter Pattern

External services (LLM, search) use adapter traits:

```rust
// Define trait in contexa-llm
pub trait LlmProvider: Send + Sync { ... }

// Implement per provider
pub struct OpenAiProvider { ... }
pub struct OllamaProvider { ... }
```

---

## 10. Security Coding Practices

- Never hardcode secrets; use OS credential vault
- Validate all external input (MCP args, user settings)
- Use parameterized SQL queries (`rusqlite` — see ADR-0010)
- Sanitize context text before LLM injection
- Run `cargo-audit` in CI for known vulnerabilities

---

## 11. Future Expansion

- Auto-generate TypeScript types from Rust structs (ts-rs)
- Custom clippy lints for Contexa-specific patterns
- Pre-commit hooks (fmt, clippy, lint)
- Code coverage reporting (tarpaulin + vitest coverage)

---

## 12. References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Tauri Security Best Practices](https://tauri.app/security/)
- [React TypeScript Cheatsheet](https://react-typescript-cheatsheet.netlify.app/)
