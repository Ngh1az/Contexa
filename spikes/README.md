## Phase 0.5 — Technical spikes

This folder contains time-boxed experiments (Phase 0.5) that validate Contexa’s riskiest assumptions **before** Phase 1 scaffolding.

Source of truth: `docs/22_Technical_Spike_Plan.md`.

### Current spikes

- `SP-01-uia-coverage/`: UIA text extraction across target apps — **Accepted** (6/7 measurable apps ≥0.8; Acrobat/Slack/Figma excluded by owner decision)
- `SP-02-capture-cpu/`: WGC capture + frame diff CPU — **Pass** (soak run in `report.md`)
- `SP-03-ocr-fallback/`: OCR fallback latency + accuracy — **Pass**
- `SP-04-sqlite-vec/`: sqlite-vec p50/p95 search latency baseline (384-dim) — **Pass**
- `SP-05-embedding/`: fastembed MRR@10 + latency — **Pass**
- `SP-06-mcp-cursor/`: MCP stdio server protocol + tool-call latency — **Pass**
- `SP-07-tauri-overlay/`: Tauri overlay + global hotkey latency baseline — **Pass**
- `SP-08-com-threading/`: COM threading patterns A/B/C — **Pass** (Pattern B selected)
- `SP-09-sqlcipher/`: SQLCipher + sqlite-vec compatibility — **Pass** (requires tuned `PRAGMA cache_size` in production — see `report.md`)

Baselines: `benchmarks/BASELINE.md`

