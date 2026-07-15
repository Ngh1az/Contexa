## Phase 0.5 — Technical spikes

This folder contains time-boxed experiments (Phase 0.5) that validate Contexa’s riskiest assumptions **before** Phase 1 scaffolding.

Source of truth: `docs/22_Technical_Spike_Plan.md`.

### Current spikes

- `SP-01-uia-coverage/`: UIA text extraction across target apps — **Partial** (3/3 measurable pass; 6 apps unavailable)
- `SP-02-capture-cpu/`: WGC capture + frame diff CPU — **Pass** (soak run in `report.md`)
- `SP-04-sqlite-vec/`: sqlite-vec p50/p95 search latency baseline (384-dim) — **Pass**
- `SP-05-embedding/`: fastembed MRR@10 + latency — **Pass**
- `SP-07-tauri-overlay/`: Tauri overlay + global hotkey latency baseline — **Pass**
- `SP-08-com-threading/`: COM threading patterns A/B/C — **Pass** (Pattern B selected)

Baselines: `benchmarks/BASELINE.md`

