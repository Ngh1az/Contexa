# Technical Spike Plan

**Project:** Contexa — AI Context Platform  
**Version:** 1.3  
**Status:** Reviewed  
**Last Updated:** 2026-07-07

---

## 1. Overview

Before full implementation (Phase 1), Contexa requires **technical spikes** — time-boxed experiments to validate risky assumptions. Each spike produces a **Spike Report** with pass/fail criteria and measured data.

**Duration:** 2 weeks (Phase 0.5)  
**Goal:** De-risk architecture decisions with empirical evidence.

---

## 2. Spike Index

| ID | Spike | Risk Addressed | Duration | Owner |
|----|-------|----------------|----------|-------|
| SP-01 | UIA text extraction coverage | R-T01: UIA gaps | 3 days | Vision |
| SP-02 | Graphics Capture + frame diff CPU | R-T02: Performance | 2 days | Vision |
| SP-03 | OCR fallback latency | R-T01, R-T02 | 2 days | Vision |
| SP-04 | sqlite-vec search at scale | R-T06: SQLite scale | 2 days | Memory |
| SP-05 | Embedding model (local) | Embedding ADR | 2 days | Memory |
| SP-06 | MCP server + Cursor integration | R-T04, ecosystem | 2 days | MCP |
| SP-07 | Tauri overlay + global hotkey | UX latency | 1 day | Desktop |
| SP-08 | COM threading model | Windows stability | 2 days | Vision |
| SP-09 | SQLCipher + sqlite-vec compatibility | R-S02, ADR-0009 | 2 days | Database |

---

## 3. SP-01: UIA Text Extraction Coverage

### Hypothesis

UI Automation extracts sufficient text (confidence > 0.8) from ≥ 80% of top user applications without OCR.

### Test Applications

| App | Process | Expected UIA Source |
|-----|---------|---------------------|
| Google Chrome | `chrome.exe` | Document content, address bar |
| VS Code | `Code.exe` | Editor text via TextPattern |
| Microsoft Word | `WINWORD.EXE` | Document body |
| Notepad | `Notepad.exe` | Edit control Value |
| Windows Terminal | `WindowsTerminal.exe` | Console text (may fail) |
| Adobe Acrobat | `Acrobat.exe` | Document text (may need OCR) |
| Slack | `slack.exe` | Message list (Electron — may fail) |
| Figma | `Figma.exe` | Canvas (expected fail — custom render) |
| Excel | `EXCEL.EXE` | Cell grid |
| Outlook | `OUTLOOK.EXE` | Email list (not body) |

### Method

1. Build minimal Rust binary using `windows` crate + `uiautomation` crate
2. For each app: focus window, walk UIA tree (depth 20), collect Name + Value
3. Measure: text length, extraction time, confidence score
4. Compare with manual copy-paste ground truth

### Pass Criteria

| Metric | Target |
|--------|--------|
| Apps with confidence ≥ 0.8 | ≥ 8 of 10 |
| Extraction time (p95) | < 150 ms |
| Text accuracy vs ground truth | > 90% character match |

### Fail Action

- If < 8/10 pass: prioritize OCR fallback tuning and app-specific enrichers
- If Terminal/Slack fail: create dedicated enrichers or accept lower confidence

### Deliverable

`spikes/SP-01-uia-coverage/report.md` with per-app results table

---

## 4. SP-02: Graphics Capture + Frame Diff CPU

### Hypothesis

Adaptive capture (1-10 fps) with frame differencing keeps CPU < 5% during active use.

### Method

1. Implement capture loop with Windows Graphics Capture API
2. Perceptual hash at 1/4 resolution
3. Run for 30 minutes across: idle (Notepad), active (Chrome browsing), interactive (VS Code typing)
4. Measure CPU with `GetProcessTimes` every 5 seconds

### Pass Criteria

| State | CPU Target | Measurement |
|-------|------------|-------------|
| Idle (1 fps) | < 1% | 30-min average |
| Active (5 fps) | < 3% | 30-min average |
| Interactive (10 fps) | < 5% | 30-min average |
| Memory | < 100 MB | Process working set |

### Deliverable

`spikes/SP-02-capture-cpu/report.md` with CPU timeline chart

---

## 5. SP-03: OCR Fallback Latency

### Hypothesis

Targeted OCR on changed regions completes in < 500ms and UIA+OCR hybrid achieves > 95% accuracy on failed UIA apps.

### Method

1. Use `Windows.Media.Ocr` via `windows` crate
2. Test on: Acrobat (PDF), Slack, Figma (expected OCR-needed apps)
3. Measure: OCR time per region, accuracy vs ground truth

### Pass Criteria

| Metric | Target |
|--------|--------|
| Single region OCR | < 500 ms |
| Hybrid accuracy (UIA fail apps) | > 90% |
| CPU spike during OCR | < 15% for < 1 second |

---

## 6. SP-04: sqlite-vec Search at Scale

### Hypothesis

Semantic search over 50K **384-dim** vectors (default fastembed path) returns results in < 200ms on reference hardware.

### Method

1. Generate 50K random 384-dim vectors (simulating 90 days of heavy use at default embedding)
2. Insert into sqlite-vec with WAL mode via rusqlite (ADR-0010)
3. Run 100 cosine similarity queries; measure p50, p95, p99
4. **Optional:** Repeat with 50K × 768-dim if quality mode is validated in SP-05

### Pass Criteria

| Metric | Target |
|--------|--------|
| Search p50 | < 100 ms |
| Search p95 | < 200 ms |
| Insert batch (10 vectors) | < 100 ms |
| Database size (50K × 384-dim) | < 200 MB |

### Fail Action

- If p95 > 200ms: evaluate vector index parameters, chunk reduction, or **Plan B: [usearch](https://github.com/unum-cloud/usearch)** (see R-T06 in [15_Risk_Analysis.md](./15_Risk_Analysis.md))

---

## 7. SP-05: Embedding Model Selection

### Hypothesis

`fastembed` + all-MiniLM-L6-v2 provides sufficient quality (MRR > 0.7) with < 0.5s batch embed time; `nomic-embed-text` via Ollama validates as quality opt-in.

### Candidates

| Model | Dimensions | Size | Runtime | Role |
|-------|------------|------|---------|------|
| `all-MiniLM-L6-v2` | 384 | 80 MB | fastembed (in-process) | **Default** |
| `nomic-embed-text` | 768 | 274 MB | Ollama local | Quality opt-in |
| `text-embedding-3-small` | 1536 | API | OpenAI cloud | Settings override only |

### Method

1. Embed 100 sample context chunks with each model
2. Run 20 search queries with known relevant chunks
3. Measure: MRR (Mean Reciprocal Rank), embed latency, memory usage

### Pass Criteria

| Metric | Target |
|--------|--------|
| MRR@10 (fastembed default) | > 0.7 |
| MRR@10 (nomic quality) | > 0.75 |
| Batch embed (10 chunks, fastembed) | < 0.5 s |
| Batch embed (10 chunks, nomic) | < 2 s |
| Model memory (fastembed) | < 200 MB |

### Decision

Document chosen model in [ADR/0006-embedding-model.md](../ADR/0006-embedding-model.md)

---

## 8. SP-06: MCP Server + Cursor Integration

### Hypothesis

Contexa MCP server works with Cursor IDE as external MCP client for `get_current_context`.

### Method

1. Implement minimal MCP server (stdio) with `get_current_context` tool
2. Configure in Cursor `mcp.json`
3. Verify: Cursor agent can call tool and receive valid context JSON
4. Measure: tool call latency

### Pass Criteria

| Metric | Target |
|--------|--------|
| Cursor recognizes server | ✅ |
| Tool call succeeds | ✅ |
| Latency | < 10 ms |
| JSON schema valid | ✅ |

---

## 9. SP-07: Tauri Overlay + Global Hotkey

### Hypothesis

Tauri overlay opens within 200ms of `Alt+Space` with preloaded WebView.

### Method

1. Tauri 2.x app with transparent overlay window
2. Register global hotkey via `tauri-plugin-global-shortcut`
3. Preload WebView on startup
4. Measure: hotkey press to overlay visible (100 iterations)

### Pass Criteria

| Metric | Target |
|--------|--------|
| Open latency p50 | < 150 ms |
| Open latency p95 | < 200 ms |
| Focus steal duration | < 100 ms |

---

## 10. SP-08: COM Threading Model

### Hypothesis

UIA and Graphics Capture work correctly when called from dedicated threads with proper COM initialization.

### Method

1. Test three patterns:
   - **A:** Single thread, `CoInitializeEx(COINIT_APARTMENTTHREADED)`
   - **B:** Capture thread (MTA) + UIA thread (STA)
   - **C:** All operations on main STA thread
2. Run 1000 capture+UIA cycles per pattern
3. Monitor: COM errors, deadlocks, access violations

### Pass Criteria

| Metric | Target |
|--------|--------|
| Zero COM errors over 1000 cycles | ✅ |
| No deadlocks | ✅ |
| Pattern with best throughput selected | Document in ADR-0008 |

### Decision

Document in [ADR/0008-windows-com-threading.md](../ADR/0008-windows-com-threading.md)

---

## 11. SP-09: SQLCipher + sqlite-vec Compatibility

### Hypothesis

`rusqlite` with `bundled-sqlcipher` can load `sqlite-vec` after `PRAGMA key` and run cosine search without errors.

### Method

1. Create encrypted DB with SQLCipher 4 defaults
2. Load sqlite-vec extension after unlock
3. Insert 1K vectors; run 50 search queries
4. Measure: startup unlock time, search p95 vs unencrypted baseline

### Pass Criteria

| Metric | Target |
|--------|--------|
| Extension loads after PRAGMA key | ✅ |
| Vector insert + search | ✅ |
| Search p95 delta vs plain DB | < +50% |
| Unlock on startup | < 100 ms |

### Fail Action

- Defer whole-DB SQLCipher to v1.2
- Ship v1.1 Pro with **Windows DPAPI column-level encryption** for sensitive text columns only

---

## 12. Spike Report Template

```markdown
# SP-XX: [Title]

**Date:** YYYY-MM-DD
**Owner:** [Name]
**Status:** Pass / Fail / Partial

## Summary
[1-2 sentences]

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| ... | ... | ... | ✅/❌ |

## Observations
[Key findings]

## Recommendation
[Proceed / Modify approach / Escalate]

## Raw Data
[Link to logs, charts]
```

---

## 13. Gate Criteria

Phase 1 (Foundation) **cannot start** until:

| Spike | Required Result |
|-------|-----------------|
| SP-01 | Pass (≥ 8/10 apps) |
| SP-02 | Pass (CPU targets) |
| SP-04 | Pass (search < 200ms at 384-dim) |
| SP-05 | Pass (fastembed MRR > 0.7; nomic documented as quality opt-in) |
| SP-07 | Pass (< 200ms overlay) |
| SP-08 | Decision recorded (ADR-0008) |

SP-03, SP-06, and SP-09 are recommended but not blocking Phase 1. **SP-09 blocks SQLCipher Pro feature** (v1.1), not GA.

---

## 14. References

- [14_Development_Roadmap.md](./14_Development_Roadmap.md)
- [15_Risk_Analysis.md](./15_Risk_Analysis.md)
- [17_Performance_Optimization.md](./17_Performance_Optimization.md)
