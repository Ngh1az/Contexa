# Contexa — Claude Code Guide

AI Context Platform (local-first, Tauri + Rust, MCP-native). **Pre-implementation:** specs in `docs/`, decisions in `ADR/`, reference clones in `reference-repos/`.

**Priority:** User message → `CLAUDE.md` → `AGENTS.md` → Superpowers skills → default behavior.

---

## Docs routing (always, before any answer or code)

User chats in natural language only — no `@file`, no slash commands. Infer the domain and **read specs before answering or coding**; never ask the user to tag files. Applies to any Contexa task: explain, review, implement, spike, design.

**Step 1 — always read:** `docs/README.md` (index + locked stack) and `AGENTS.md`.

**Step 2 — domain spec (Read, don't guess):**

| If task mentions… | Read |
|-------------------|------|
| Vision, UIA, OCR, capture, SP-01/02/03/08 | `docs/05`, `ADR/0002`, `ADR/0008`, `docs/22` |
| Context, enrichment, plugins | `docs/06`, `docs/18` |
| Memory, timeline, embedding, search, SP-04/05 | `docs/07`, `docs/04`, `ADR/0006`, `docs/22` |
| Database, rusqlite, migrations, SQLCipher, SP-09 | `docs/04`, `ADR/0003`, `ADR/0009`, `ADR/0010` |
| MCP, tools, resources, SP-06 | `docs/11`, `ADR/0004` |
| Orchestrator, LLM, Ollama | `docs/08`, `ADR/0007` |
| Search, web | `docs/09`, `ADR/0011` |
| Rerank | `ADR/0012`, `docs/28` |
| Prompt, tokens | `docs/10` |
| UI, overlay, hotkey, SP-07 | `docs/12`, `docs/03` |
| IDE / LSP / tree-sitter | `docs/27`, `ADR/0013` |
| Requirements FR/NFR | `docs/01` |
| Tests, benchmarks | `docs/13` |
| Security, privacy | `docs/16`, `docs/25` |
| Roadmap, phases, future tech | `docs/14`, `docs/28` |
| Any spike SP-XX | `docs/22` (matching section only) |

**Step 3 — reference repos:** map in `docs/26_Reference_Repos.md`. Grep for the pattern you need, then read 1–3 files — never bulk-read a repo. Patterns only; no code copying without license review.

**Step 4 — stack changes:** proposing a different crate, DB layer, or embedding default requires checking `ADR/` first. If no ADR exists, say so before implementing. Locked: rusqlite + refinery (not sqlx), fastembed 384-dim default, rmcp, Tauri 2.x.

**Step 5 — UI work only:** load the `design-taste-frontend` skill from `.agents/skills/` for React overlay work (not for Rust engines).

Lightweight tasks (typo, changelog-only): Step 1 still applies; skip Step 3.

---

## Ponytail — lazy senior dev (always on)

Lazy means efficient, not careless. The best code is the code never written. Before writing code, stop at the first rung that holds:

1. Does this need to exist at all? (YAGNI)
2. Already in this codebase? Reuse it.
3. Stdlib does it? Use it.
4. Native platform feature? Use it.
5. Installed dependency solves it? Use it.
6. Can it be one line? Make it one line.
7. Only then: minimum code that works.

Climb the ladder **after** understanding the problem: read the task and code it touches, trace the real flow end to end. Bug fix = root cause, not symptom — grep every caller and fix the shared function once.

Rules: no unrequested abstractions, dependencies, or boilerplate. Deletion over addition; boring over clever; shortest working diff — but the smallest change in the wrong place is a second bug. Prefer the edge-case-correct option when two approaches are the same size. Mark intentional shortcuts with a `ponytail:` comment naming the ceiling and upgrade path.

Never lazy about: understanding the problem, input validation at trust boundaries, error handling that prevents data loss, security, accessibility, anything explicitly requested. Non-trivial logic leaves ONE runnable check behind (assert-based self-check or one small test file — no frameworks); trivial one-liners need none.

## Karpathy guidelines (always on)

1. **Think before coding** — state assumptions; if multiple interpretations exist, present them, don't pick silently; push back when a simpler approach exists; if confused, stop and ask.
2. **Simplicity first** — nothing speculative; no flexibility nobody asked for; "would a senior engineer call this overcomplicated?"
3. **Surgical changes** — touch only what you must; match existing style; don't improve adjacent code; remove only orphans YOUR change created; mention (don't delete) pre-existing dead code.
4. **Goal-driven** — turn tasks into verifiable goals ("fix bug" → "write repro test, make it pass"); for multi-step work state a brief plan with a verify check per step.

---

## Testing & phase gates

- **Spikes:** one minimal runnable check per spike (see `docs/22`). **Production:** TDD for core logic.
- Verification required before claiming a spike pass or phase done — match gates in `docs/22_Technical_Spike_Plan.md`.
- Phase 0.5 spikes must pass before Phase 1 scaffolding. Record baselines in `benchmarks/BASELINE.md`.
- Debug UIA/COM/sqlite-vec failures systematically (hypothesis → minimal repro → fix).

## Superpowers workflow skills (`.agents/skills/`)

Installed via `npx skills add obra/superpowers`. **This file and project rules override** when they conflict:

| Superpowers skill | Contexa override |
|-------------------|------------------|
| `brainstorming` | Use for new engines/features; skip for trivial doc fixes |
| `writing-plans` / `executing-plans` | Use for Phase 1+ and multi-crate work |
| `test-driven-development` | **Spikes:** one minimal runnable check per spike (see `docs/22`). **Production:** TDD for core logic; ponytail allows assert-based checks for spikes |
| `verification-before-completion` | **Required** before claiming spike pass or phase done — match gates in `docs/22` |
| `systematic-debugging` | Use for UIA/COM/sqlite-vec failures |
| `subagent-driven-development` / `dispatching-parallel-agents` | OK for parallel spikes (SP-01 + SP-04) |

## Other skills (`.agents/skills/`)

| Skill | Use for |
|-------|---------|
| `design-taste-frontend` | React overlay UI — anti-slop frontend (primary design skill) |
| `web-design-guidelines` | Reviewing overlay UI — accessibility/UX rules |
| `vercel-react-best-practices` | React overlay performance (ignore Next.js-specific rules — Contexa is Tauri + React SPA) |
| `find-skills` | Discovering new skills when a task type is uncovered |
| `understand*` (8 skills) | Codebase knowledge graph, dashboard, diff impact, onboarding |

## Quick doc map

| Domain | Spec | ADR | Reference repos |
|--------|------|-----|-----------------|
| Vision | `docs/05` | 0002, 0008 | `screenpipe`, `windows-rs`, `tier2/uiautomation-rs` |
| Context | `docs/06` | 0005 | `screenpipe` |
| Memory | `docs/07` | 0003, 0006, 0012 | `sqlite-vec`, `tier2/fastembed-rs` |
| Database | `docs/04` | 0003, 0009, 0010 | `sqlite-vec` |
| MCP | `docs/11` | 0004 | `rust-sdk`, `tier3/mcp-servers` |
| Orchestrator / LLM | `docs/08` | 0007 | `ollama`, `tier2/ollama-rs` |
| Search | `docs/09` | 0011, 0012 | — |
| UI / Shell | `docs/12`, `docs/03` | 0001 | `tauri`, `tier2/plugins-workspace`, `tier2/shadcn-ui` |
| IDE / LSP | `docs/27` | 0013 | — |
| Spikes | `docs/22` | — | per spike table in `docs/26` |

Full index: `docs/README.md` · Repo map: `docs/26_Reference_Repos.md`

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
