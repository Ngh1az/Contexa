# ADR-0008: Windows COM Threading Model

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team  
**Validates:** SP-08 in [22_Technical_Spike_Plan.md](../docs/22_Technical_Spike_Plan.md)

---

## Context

Windows UI Automation (UIA) and Windows Graphics Capture (WGC) are COM-based APIs with strict threading requirements:

- UIA (`IUIAutomation`) requires **Single-Threaded Apartment (STA)**
- WGC (`GraphicsCaptureItem`) can run in **Multi-Threaded Apartment (MTA)**
- Calling COM objects on the wrong thread causes `RPC_E_WRONG_THREAD` errors or silent failures
- Tauri main thread is STA (WebView2 requirement)

## Decision

Use a **dedicated STA capture thread** that owns all UIA and WGC COM objects. Graphics Capture frames are passed to this thread via a bounded channel. OCR runs on a separate STA thread pool (max 2 workers).

```
Main Thread (STA) ─── Tauri + WebView2
Capture Thread (STA) ─── Window Monitor + WGC + UIA
OCR Thread Pool (STA, 1-2) ─── Windows.Media.Ocr
Tokio Runtime ─── Async (LLM, search, DB, MCP)
```

## Implementation

```rust
// Capture thread initialization
std::thread::Builder::new()
    .name("contexa-capture".into())
    .spawn(|| {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .expect("COM init failed");
        }
        
        let automation: IUIAutomation = create_uia_instance();
        let capturer = GraphicsCapturer::new();
        
        capture_loop(automation, capturer);
        
        unsafe { CoUninitialize() };
    });
```

## Rules

| Rule | Rationale |
|------|-----------|
| UIA objects never cross thread boundaries | COM apartment safety |
| WGC frames copied to owned buffer before channel send | `Send` safety |
| OCR engine created per-thread (not shared) | COM apartment safety |
| Main thread never calls UIA or WGC directly | Avoid blocking WebView |
| `CoInitializeEx` called once per thread at start | Required COM setup |
| Thread named for debugging (`contexa-capture`) | Traceability |

## Alternatives Considered

| Pattern | Verdict |
|---------|---------|
| All on main STA thread | Rejected — blocks WebView during UIA walks |
| MTA for everything | Rejected — UIA requires STA |
| `CoMarshalInterThreadInterfaceInStream` | Rejected — unnecessary complexity for our pipeline |
| Process-per-engine | Rejected — IPC overhead too high |

## Consequences

**Positive:**
- Stable COM usage with zero cross-thread violations
- Capture pipeline isolated from UI responsiveness
- Clear thread ownership model

**Negative:**
- Frame data must be copied between threads (memory overhead ~10 MB)
- STA thread pool limited to 1-2 OCR workers (by design)
- Debugging requires thread-aware logging

## Validation (SP-08, 2026-07-14)

Spike SP-08 ran 1000 UIA cycles + 1000 WGC frames under three patterns — all with **zero COM errors and no deadlocks**:

| Pattern | Result |
|---------|--------|
| A — single STA thread owns UIA + WGC (this ADR's design) | ✅ validated |
| B — capture MTA thread ∥ UIA STA thread | ✅ validated (fallback if UIA walks starve frame draining) |
| C — main STA thread | ✅ works, remains rejected (blocks WebView) |

Notes: `Direct3D11CaptureFramePool::CreateFreeThreaded` works from both STA and MTA — no DispatcherQueue required. UIA shallow ops ~0.7–0.8k ops/s; full-window walks ~112 ms p95 (SP-01). See `spikes/SP-08-com-threading/report.md`.

## References

- [05_Vision_Engine.md](../docs/05_Vision_Engine.md)
- [02_System_Architecture.md](../docs/02_System_Architecture.md)
- [Microsoft COM Threading](https://learn.microsoft.com/en-us/windows/win32/com/processes--threads--and-apartments)
