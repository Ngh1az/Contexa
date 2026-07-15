# ADR-0001: Rust Core with Tauri Shell

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team

---

## Context

Contexa requires a desktop application that runs continuously in the background with low resource usage, direct access to Windows APIs (UI Automation, Graphics Capture), and a modern UI for the overlay and settings.

We need to choose between:
- **Electron** — JavaScript/TypeScript with Chromium
- **Tauri** — Rust backend with system WebView
- **Native WinUI/WPF** — C# with Windows-native UI
- **Flutter** — Dart with custom rendering

## Decision

Use **Tauri 2.x** as the desktop shell with a **Rust core** for all engines and a **React/TypeScript** frontend for the overlay UI.

## Rationale

| Factor | Tauri + Rust | Electron | WinUI |
|--------|-------------|----------|-------|
| Binary size | ~10 MB | ~150 MB | ~5 MB |
| Memory usage | ~50 MB (WebView) | ~200 MB (Chromium) | ~30 MB |
| Windows API access | Direct (Rust crates) | Via native modules | Native |
| UI flexibility | React + TailwindCSS | React + TailwindCSS | XAML |
| Cross-platform (future) | Yes (macOS, Linux) | Yes | Windows only |
| Performance | Native speed | V8 overhead | Native speed |
| Ecosystem | Growing | Mature | Windows-only |

Rust provides memory safety and performance for the capture pipeline without GC pauses. Tauri provides a lightweight shell with system WebView (Edge WebView2 on Windows), avoiding the Chromium bundle overhead of Electron.

## Consequences

**Positive:**
- Small binary size and low memory footprint
- Direct Windows API access via `windows` crate
- Rust's concurrency model suits multi-threaded engine architecture
- React UI ecosystem for rapid overlay development

**Negative:**
- Smaller Tauri ecosystem compared to Electron
- Rust compile times in development
- WebView2 dependency on Windows (mitigated by bundling bootstrapper)
- Team needs Rust expertise

## References

- [02_System_Architecture.md](../docs/02_System_Architecture.md)
- [Tauri Documentation](https://tauri.app/)
