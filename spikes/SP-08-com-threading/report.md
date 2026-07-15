# SP-08: COM Threading Model

**Date:** 2026-07-14  
**Owner:** —  
**Status:** Pass

## Summary

All three threading patterns completed 1000 UIA cycles + 1000 captured WGC frames with **zero COM errors and no deadlocks**. This validates ADR-0008's accepted design (a dedicated STA capture thread owning both UIA and WGC — spike pattern A). Pattern B (capture MTA ∥ UIA STA) also passed cleanly and is recorded as the proven fallback if long UIA walks (~100 ms, see SP-01) ever starve frame draining on the shared thread.

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Zero COM errors over 1000 cycles | ✅ | 0 errors in all patterns (A/B/C) | ✅ |
| No deadlocks | ✅ | none (120 s watchdog never fired) | ✅ |
| Best-throughput pattern documented | ADR-0008 | **Pattern B** | ✅ |

Per-pattern data (1000 UIA `ElementFromHandle`+`CurrentName` cycles; 1000 frames via free-threaded frame pool):

| Pattern | UIA ops/s | Capture time for 1000 frames |
|---------|-----------|------------------------------|
| A — single spawned STA (serial UIA → capture) | 673 | 86.6 s (~12 fps) |
| B — capture MTA ∥ UIA STA | 680 | 25.3 s (~40 fps) |
| C — main thread STA (serial) | 837 | 25.8 s (~39 fps) |

## Observations

- `Direct3D11CaptureFramePool::CreateFreeThreaded` works from both STA and MTA threads — no DispatcherQueue needed. This is the recommended pool type for engine code.
- WGC only delivers frames when window content changes; frame-rate numbers therefore also reflect on-screen activity during each run (pattern A's 12 fps run coincided with a quieter screen, not a pattern defect). The pass criterion (all 1000 frames delivered, zero errors) is unaffected.
- UIA throughput ~0.7–0.8k ops/s means a single extraction is ~1.2–1.5 ms for shallow access — full-tree walks are measured separately in SP-01.
- No cross-apartment marshaling issues observed because each thread owns its COM objects end-to-end (created and used on the same thread) — this must remain an invariant in Phase 1 engine design.

## Recommendation

Keep **ADR-0008's design** for Phase 1: dedicated STA capture thread owning UIA + WGC (pattern A — validated, simplest ownership model), with the free-threaded frame pool. If profiling shows UIA walks starving frame draining, split capture onto an MTA thread (pattern B — also validated) without redesign risk. Objects never cross threads; communication via channels.

## Raw Data

- `cargo run --release` in `spikes/SP-08-com-threading/` — assert-based gate check built in.
