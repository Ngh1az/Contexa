# Contexa — Agent Guide

AI Context Platform (local-first, Tauri + Rust, MCP-native). **Pre-implementation:** specs in `docs/`, decisions in `ADR/`, reference clones in `reference-repos/`.

**Primary agent guide: `CLAUDE.md`** (docs routing, ponytail/karpathy discipline, prompt-suggestion format, phase gates, quick doc map). This file is a thin pointer kept for non-Claude agents.

## Before any implementation or architecture answer

1. Follow the **docs routing** section in `CLAUDE.md` — read the relevant `docs/` and `ADR/` files **before** coding.
2. Study `reference-repos/` for patterns only; do not copy code without license review.
3. Locked stack: see `docs/README.md` Tech Stack (rusqlite, fastembed default, MCP via rmcp).

## Code discipline (always on)

- **Ponytail** — YAGNI, minimal diff, reuse existing code (full text in `CLAUDE.md`)
- **Karpathy guidelines** — think first, surgical edits, verifiable goals (full text in `CLAUDE.md`)

## Phase gates

- **Phase 0.5 spikes** must pass gates in `docs/22_Technical_Spike_Plan.md` before Phase 1 scaffolding
- Record baselines in `benchmarks/BASELINE.md` after spikes
- Verification required before claiming spike pass or phase done

## UI overlay

- Invoke **design-taste-frontend** skill (`.agents/skills/`) when building React overlay — not for Rust engines

**Priority:** User message → `CLAUDE.md` → `AGENTS.md` → default prompt.

Full index: `docs/README.md` · Repo map: `docs/26_Reference_Repos.md`
