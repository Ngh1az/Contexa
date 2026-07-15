# Changelog

All notable documentation changes for Contexa.

## [1.3.11] — 2026-07-15

### Added — Phase 1: Vision Engine (`contexa-vision`)
- Pure-logic modules (real `cargo test` coverage, 23 tests): `ExclusionFilter`, perceptual hash/hamming (ported from `spikes/SP-02-capture-cpu`), `FrameDifferencer`, `RegionHashCache`, `AdaptiveScheduler` (Idle/Active/Interactive state machine, `Instant`-driven for testability)
- WinAPI/COM/GPU modules (verified via `examples/vision_smoke.rs` against a real window, not `cargo test`): `WindowMonitor` (`GetForegroundWindow`/`GetWindowTextW`/`QueryFullProcessImageNameW`), `UiaExtractor` (`walk()`/`confidence()` ported from `spikes/SP-01-uia-coverage`), `FrameCapturer` (WGC `Capturer` ported from `spikes/SP-02-capture-cpu`)
- `VisionEngine` trait + `ContexaVisionEngine`: dedicated STA capture thread per ADR-0008 Pattern A, bounded result channel (docs/05 §9), one-shot `capture_active_window`/`extract_uia_text` via short-lived per-call COM threads
- `OcrEngine` is an explicit stub (`ocr_region` always errors) — `SP-03-ocr-fallback` was never run; implementing real OCR now would skip the project's spike-first gate
- `contexa_core::CaptureMethod` reused for `VisionResult` (as `Option<CaptureMethod>`) instead of a second, conflicting enum
- Verified live: WGC capture succeeded against a real foreground window (frame hash produced); UIA extraction correctly failed against an Electron-based app, consistent with known UIA limitations from SP-01
- `docs/14_Development_Roadmap.md` — Vision Engine marked done (except OCR)

## [1.3.10] — 2026-07-15

### Added — Phase 1: Database Layer (`contexa-db`)
- `contexa-core`: `ContexaError`/`Result` (docs/19 §4.5) and `ContextSnapshot`/`CaptureMethod` shared types (docs/02 §8.1)
- `crates/contexa-db/migrations/V1__initial_schema.sql` — all v1.0 tables from docs/04 §5.1–5.9 (v1.1 tables deferred per §5.10)
- `Database` (WAL, sqlite-vec extension loading via the vendored `vec0.dll`, single writer + 4-connection read pool, refinery migration runner)
- `ContextRepository`, `MemoryRepository`, `TimelineRepository` (docs/04 §8.1) with rusqlite-backed implementations, incl. KNN semantic search (validated pattern from `spikes/SP-04-sqlite-vec`) and retention purge (docs/04 §7.4)
- 3 integration tests (`crates/contexa-db/tests/database.rs`) against the real vendored sqlite-vec extension — all pass
- Workspace deps: `async-trait`, `tempfile` (dev)
- `docs/14_Development_Roadmap.md` — Database Layer marked done

## [1.3.9] — 2026-07-15

### Added — Phase 0 scaffolding (unblocked by passed 0.5 gate spikes)
- `git init` + root `.gitignore`
- Cargo workspace root: `Cargo.toml` (10 crate members + `apps/desktop/src-tauri`, workspace lints, centralized `[workspace.dependencies]` pinned to versions validated in spikes), `rust-toolchain.toml`, `rustfmt.toml`
- `crates/contexa-{core,vision,context,memory,orchestrator,search,prompt,mcp,llm,db}/` — empty stub crates wired per the dependency graph in docs/02 §5.2 (no engine logic yet — Phase 1 work)
- `apps/desktop/` — Tauri skeleton adapted from the already-validated `spikes/SP-07-tauri-overlay/sp07-app` (renamed to `contexa`/`dev.contexa.app`, benchmark code stripped, preloaded overlay + Alt+Space toggle kept)
- Root `package.json` + `pnpm-workspace.yaml` (pnpm workspace, `apps/*`)
- `.github/workflows/pr-check.yml` — rust (fmt/clippy/test/build) + frontend (typecheck) jobs; release/signing pipeline deferred to Phase 5
- `docs/29_Dev_Environment_Setup.md` — prerequisites, bootstrap, run, CI-equivalent checks
- `docs/14_Development_Roadmap.md` — checked off all 4 remaining Phase 0 deliverables

## [1.3.8] — 2026-07-14

### Added — Phase 0.5 spike execution
- `spikes/SP-05-embedding/` — **Pass**: MRR@10 0.958, batch embed 27 ms, +124 MB (fastembed all-MiniLM-L6-v2)
- `spikes/SP-08-com-threading/` — **Pass**: 0 COM errors across patterns A/B/C × 1000 cycles; Pattern B selected (per ADR-0008)
- `spikes/SP-02-capture-cpu/` — **Pass** (60 s runs + 30-min soak): CPU 0.01–0.14% of machine, mem ≤ 56 MB
- `spikes/SP-01-uia-coverage/` — **Partial Pass**: 6/7 available apps ≥ 0.8 (Chrome/Word/Excel/Outlook/Notepad 1.0, Terminal 0.85); VS Code fails UIA (Monaco opaque → docs/27 LSP path justified); Acrobat/Slack/Figma not installed
- `benchmarks/BASELINE.md` — M0.5 baselines recorded per docs/17 §18
- VS 2022 Build Tools (C++ workload) installed via winget — MSVC was missing

## [1.3.7] — 2026-07-14

### Added
- Tier 2 clones (6): `windows-capture`, `img_hash`, `win-ocr-rs`, `oneocr-rs`, `ocrs`, `extism`
- Tier 3 clones (3): `anthropics/skills`, `obra/superpowers`, `awesome-claude-code`
- Superpowers skills reinstalled via `npx skills add obra/superpowers` → `.agents/skills/` (symlinked to `.claude/skills/`)
- New skills: `find-skills`, `vercel-react-best-practices`, `web-design-guidelines`
- `CLAUDE.md`: restored Superpowers override table; new skills table
- `docs/26_Reference_Repos.md`: new Tier 2/3 rows

## [1.3.6] — 2026-07-14

### Changed
- Migrated `.cursor/rules/` content into **`CLAUDE.md`** (Claude Code auto-loads it): docs routing, ponytail, karpathy, prompt suggestions, phase gates
- `AGENTS.md` slimmed to a thin pointer at `CLAUDE.md` (Superpowers/Cursor sections removed — Claude Code only)

### Removed
- `.cursor/` folder (Cursor IDE no longer used)

## [1.3.5] — 2026-07-14

### Added
- `docs/28_Tech_Expansion_Plan.md` — future-tech list mapped to phases/gates (Sentry, Plausible, mimalloc, rerank, ONNX, JetBrains, tree-sitter, WASM; GPU/SIMD conditional)
- `ADR/0012` (Proposed) — local reranking via fastembed cross-encoder; cloud rerankers rejected on privacy
- `ADR/0013` (Proposed) — tree-sitter fallback parsing for non-extension editors, gated evaluation v1.2+
- Updated indexes: `docs/README.md`, `ADR/README.md` (next ADR: 0014)

## [1.3.4] — 2026-07-08

### Added
- **Understand-Anything** (Egonex AI): 8 skills in `.agents/skills/understand*`
- Built plugin core in `reference-repos/tier3/understand-anything/`
- User junction `%USERPROFILE%\.understand-anything-plugin` → plugin package root

## [1.3.3] — 2026-07-08

### Added
- **`AGENTS.md`** — project agent guide, Superpowers overrides, doc quick map
- **`.cursor/rules/contexa-docs-routing.mdc`** — auto-read docs/ADR/repos before implement
- **Glob rules:** vision, context, memory, database, mcp, orchestrator, ui
- Updated `.cursor/rules/README.md`

## [1.3.2] — 2026-07-08

### Added
- **Cursor rules:** `ponytail` (DietrichGebert/ponytail), `karpathy-guidelines` (multica-ai/andrej-karpathy-skills)
- **Reference clone:** `reference-repos/tier3/awesome-mcp-servers` (punkpeye — MCP ecosystem directory)

## [1.3.1] — 2026-07-08

### Added
- **Tier 2** reference clones (7): `uiautomation-rs`, `sqlx`, `ollama-rs`, `fastembed-rs`, `plugins-workspace`, `shadcn-ui`, `vercel-ai`
- **Tier 3** reference clones (3): `mcp-servers`, `mem0`, `async-openai`
- `reference-repos/tier2/`, `reference-repos/tier3/` layout
- Updated `docs/26_Reference_Repos.md` with Tier 2/3 tables and local paths

## [1.3] — 2026-07-07

### Added
- `docs/26_Reference_Repos.md` — GitHub repos mapped to engines
- `ADR/0010` — rusqlite + refinery as sole DB access layer
- `ADR/0011` — DuckDuckGo as default search provider
- `SP-09` — SQLCipher + sqlite-vec compatibility spike
- `R-T07` — sqlite-vec alpha stability risk + usearch Plan B

### Changed
- **ADR-0006 amended:** fastembed (384-dim) default; nomic-embed-text quality opt-in
- Embedding schema: `embeddings` = 384-dim; `embeddings_768` = quality mode
- Removed `sqlx` as SQLite option; standardized on `rusqlite`
- Updated crate versions (`rmcp` 1.x, `fastembed` 5.x, `uiautomation` 0.25+)
- Token counting: per-provider adapters (not tiktoken-only)
- ER diagram + §5.10 v1.1 tables in Database Design
- All engine docs synced to v1.3 Reviewed

### Fixed
- Duplicate security line in Database Design §11
- ADR-0003 SQLCipher cross-reference (now points to ADR-0009)
- Storage estimates aligned with 384-dim default path
- Missing doc numbering gap (added doc 26)

## [1.2] — 2026-07-07

- v1.1 features: Hierarchical Memory, MCP Resources, IDE LSP, Entity Linking, SQLCipher
- `docs/27_IDE_LSP_Integration.md`
- ADR-0009 SQLCipher

## [1.1] — 2026-07-06

- Competitive analysis, spike plan, glossary, business model, privacy draft
- ADRs 0006–0008

## [1.0] — 2026-07-06

- Initial documentation set (docs 00–20, ADRs 0001–0005)
