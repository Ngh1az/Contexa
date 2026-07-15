# Development Environment Setup

**Project:** Contexa — AI Context Platform
**Version:** 1.0
**Status:** Reviewed
**Last Updated:** 2026-07-15

---

## 1. Overview

Steps to get a working Contexa dev environment on Windows and run the Phase 0 desktop skeleton (`apps/desktop`). See [CLAUDE.md](../CLAUDE.md) and [AGENTS.md](../AGENTS.md) for docs-routing and coding-discipline rules that apply to all changes.

---

## 2. Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | Stable (pinned via `rust-toolchain.toml`) | Install via [rustup](https://rustup.rs/); MSRV 1.75.0 |
| Node.js | 20 LTS | |
| pnpm | 9.x+ (pinned via `packageManager` in `package.json`) | `corepack enable` or `npm i -g pnpm` |
| Tauri CLI | 2.x | Installed as a dev dependency (`pnpm -C apps/desktop tauri --version`) |
| VS 2022 Build Tools (C++ workload) | Latest | Required for MSVC linker; `winget install Microsoft.VisualStudio.2022.BuildTools` |
| WebView2 | Evergreen | Preinstalled on Windows 11; bundled in the installer otherwise |

---

## 3. Bootstrap

```powershell
# Rust workspace deps
cargo fetch

# Node workspace deps (root — resolves apps/desktop via pnpm-workspace.yaml)
pnpm install
```

---

## 4. Run

```powershell
pnpm -C apps/desktop tauri dev
```

Preloaded overlay window; press `Alt+Space` to toggle visibility (validated in `spikes/SP-07-tauri-overlay`, open latency p50 5ms / p95 9ms — see `benchmarks/BASELINE.md`).

---

## 5. Checks (same as CI — `.github/workflows/pr-check.yml`)

```powershell
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
pnpm -C apps/desktop typecheck
```

---

## 6. Repository Layout

See [docs/19_Coding_Standards.md](./19_Coding_Standards.md) §3 for the full target structure. Current state after Phase 0 scaffolding:

```
contexa/
├── apps/desktop/           # Tauri app (Rust + React) — skeleton only, no engine wiring yet
├── crates/contexa-*/       # 10 engine crates — empty stubs, Phase 1 fills these in
├── Cargo.toml              # Workspace root
├── package.json            # Node workspace root (pnpm)
├── pnpm-workspace.yaml
└── .github/workflows/      # CI (PR checks only — no release pipeline yet)
```

`apps/web` (Next.js marketing site) and the release/signing CI pipeline are intentionally not scaffolded yet — see `docs/14_Development_Roadmap.md` for phase timing.

---

## 7. References

- [14_Development_Roadmap.md](./14_Development_Roadmap.md)
- [19_Coding_Standards.md](./19_Coding_Standards.md)
- [20_Deployment.md](./20_Deployment.md)
- [22_Technical_Spike_Plan.md](./22_Technical_Spike_Plan.md)
