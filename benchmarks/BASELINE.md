# Benchmark Baselines — M0.5 (Post-Spike)

**Hardware:** 32 logical cores, Windows 11 Home 26200 (note: differs from reference i5-12400/16GB in docs/17 §18 — re-baseline on reference hardware before Alpha SLA)  
**Date:** 2026-07-14  
**Protocol:** docs/17 §18 (3-run median where applicable)

## Gate Spike Results

| Spike | Metric | Target | Measured | Status |
|-------|--------|--------|----------|--------|
| SP-04 | Search p50 (50K × 384-dim) | < 100 ms | 51 ms | ✅ |
| SP-04 | Search p95 | < 200 ms | 57 ms | ✅ |
| SP-04 | Insert batch (10 vectors) | < 100 ms | 2 ms | ✅ |
| SP-04 | DB size (50K × 384-dim) | < 200 MB | 75.09 MB | ✅ |
| SP-05 | MRR@10 (fastembed 384) | > 0.7 | 0.958 | ✅ |
| SP-05 | Batch embed 10 chunks | < 500 ms | 27 ms | ✅ |
| SP-05 | Model memory delta | < 200 MB | +124 MB | ✅ |
| SP-07 | Overlay open p50 | < 150 ms | 5 ms | ✅ |
| SP-07 | Overlay open p95 | < 200 ms | 9 ms | ✅ |
| SP-08 | COM errors (3 patterns × 1000 cycles) | 0 | 0 | ✅ |
| SP-08 | UIA shallow extraction throughput | — | 673–837 ops/s | baseline |
| SP-02 | CPU idle 1 fps (10-min soak, frames flowing) | < 1% | 0.02% of machine | ✅ |
| SP-02 | CPU active 5 fps (60 s run, frames flowing) | < 3% | 0.05% of machine | ✅ |
| SP-02 | CPU interactive 10 fps (60 s run, frames flowing) | < 5% | 0.14% of machine | ✅ |
| SP-02 | Capture memory | < 100 MB | 55 MB | ✅ |
| SP-01 | UIA coverage ≥ 0.8 confidence | ≥ 8/10 apps | 6/7 measurable, 86% (Acrobat/Slack/Figma excluded — owner decision, not installing test apps; VS Code fails → LSP extension path) | ✅ accepted |
| SP-01 | UIA full-window walk p95 | < 150 ms | ~110 ms non-Office; 200–400 ms Office (enricher mitigation specced) | ⚠️ pass w/ note |

## Non-Gate Spikes

| Spike | Status |
|-------|--------|
| SP-03 (OCR fallback) | Pass (2026-07-16) — latency p95 19-106ms (target <500ms), accuracy 98.2% vs. planted ground truth (target >90%), CPU 1.67% of machine over 10 calls (target <15%). Acrobat/Slack/Figma still unavailable on this machine (same gap as SP-01); validated against a Notepad ground-truth window instead. See `spikes/SP-03-ocr-fallback/report.md`. |
| SP-06 (MCP + Cursor) | Pass (2026-07-18) — `rmcp` 0.8 stdio server: tool listed with valid JSON Schema, tool call succeeds, latency p50=0.16ms/p95=0.24ms/p99=0.30ms over persistent connection (target <10ms). Verified via MCP Inspector CLI + a `TokioChildProcess` Rust client (same stdio launch mechanism Cursor uses); real Cursor IDE is installed on this machine but `~/.cursor/mcp.json` was intentionally left untouched (owner decision) rather than adding a throwaway spike server to the live config. See `spikes/SP-06-mcp-cursor/report.md`. |
| SP-09 (SQLCipher + sqlite-vec) | Pass (2026-07-18) — extension loads after `PRAGMA key`, insert/search correct, unlock <1ms (target <100ms) at all scales. **Root cause found and fixed**: with SQLite's default `cache_size` (~2MB), un-indexed `vec0` KNN full-table scans forced repeated page decryption, causing +254–300% search p95 regression at 10K–50K vectors (target <+50%). Sizing `PRAGMA cache_size` to the working set fixes it entirely — encrypted search then measured **58–71% faster** than plain at every scale (1K/10K/50K, 3× repeated at 50K). Production requirement: `contexa-db` must tune `cache_size`/`mmap_size` at connection open, not rely on SQLite's default. See `spikes/SP-09-sqlcipher/report.md`, ADR-0009 SP-09 update. |

## Notes

- SP-02 single-core equivalents: 0.44% / 1.46% / 4.40% at 1/5/10 fps. Task-manager style (÷32 cores) reported above; on reference 12-thread hardware these would be ~0.04% / 0.12% / 0.37% — comfortably inside targets.
- SP-02 was measured against a window with real activity (streaming text); WGC delivers frames only on content change, so idle windows cost ~0.
- SP-01 lessons recorded for Phase 1: gate UIA pattern queries on control type (2× speedup); early-stop at char budget (Excel 2070→220 ms); find windows via `EnumWindows`+`ElementFromHandle` (UIA root-tree walk misses elevated/Electron windows); VS Code/Monaco is UIA-opaque → title enricher (v1.0) + LSP extension (v1.1, docs/27); new Outlook = `olk.exe`.
- SP-01 closed as accepted (2026-07-18) without installing Acrobat/Slack/Figma — owner call; expected outcomes for those apps were already anticipated by the spec's fail-action and are covered by existing mitigations (OCR fallback, enrichers), so re-running was judged to add no new architectural information.
- SP-08 decision: ADR-0008 design validated (single STA capture thread owning UIA + WGC, free-threaded frame pool); pattern B (capture MTA ∥ UIA STA) validated as fallback.
