# SP-07: Tauri Overlay + Global Hotkey

**Date:** 2026-07-08  
**Owner:**  
**Status:** Pass

## Summary

Preloaded Tauri 2 overlay (hidden at startup) meets open-latency and focus targets. Measured open path is `show()` + `set_focus()` after hotkey receipt; global hotkey `Alt+Space` toggles visibility via `tauri-plugin-global-shortcut`.

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Open latency p50 | < 150 ms | 5 ms | ✅ |
| Open latency p95 | < 200 ms | 9 ms | ✅ |
| Focus steal duration | < 100 ms | p50=11 ms, p95=13 ms | ✅ |

## Observations

- Window is created at startup with `visible: false` (preload path); hotkey only shows/hides + focuses.
- Benchmark is programmatic 100× show/hide (proxy for “hotkey already handled → visible”), not OS-level input event timing.
- `Alt+Space` can conflict with Windows system menu on some focus states; note if manual toggle felt flaky in daily use.
- Focus-steal metric here is time from `show()` return to `set_focus()` return (spike proxy), not a Windows focus-steal API timing.

## Recommendation

- Proceed with Tauri 2 + preloaded overlay + `tauri-plugin-global-shortcut` for Phase 1 shell.
- If production hotkey conflicts with OS, pick an alternate default (e.g. `Ctrl+Space` / configurable) while keeping preload architecture.

## Raw Data

- Command: in-app button **Run open-latency bench (100×)** → `run_open_latency_bench`
- Output:
  ```
  iterations=100
  open_latency_ms: p50=5, p95=9
  focus_steal_ms: p50=11, p95=13
  ```
- App: `spikes/SP-07-tauri-overlay/sp07-app/`
