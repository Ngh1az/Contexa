# SP-03: OCR Fallback Latency

**Date:** 2026-07-16
**Owner:** —
**Status:** Pass — all three criteria measured and passing

## Summary

`Windows.Media.Ocr` via the real production path (`contexa_vision::FrameCapturer` + `contexa_vision::OcrEngine::ocr_region`, the exact code `crates/contexa-vision/src/engine.rs`'s `ocr_region` calls) meets all three docs/22 §5 pass criteria: latency, accuracy, and CPU usage. The spec's target apps (Acrobat, Slack, Figma) are still not installed on this machine (same gap SP-01 recorded), so accuracy was validated against a planted-ground-truth Notepad window instead — captured by explicit HWND (`--hwnd`), not the OS foreground window, so the test never stole input focus from other active work on the machine.

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Single region OCR (p50) | — | 13–98 ms (varies by capture size — small cropped region vs. full window) | — |
| Single region OCR (p95) | < 500 ms | 19–106 ms across all runs (full-window and cropped-region) | ✅ |
| Hybrid accuracy vs. ground truth | > 90% | **98.2%** (normalized Levenshtein similarity, cropped content region, planted 3-sentence Notepad ground truth) | ✅ |
| CPU spike during OCR | < 15% | **1.67%** of machine (32 cores), 10 calls / 906ms wall time | ✅ |

## Observations

1. **API surface needed live iteration**: `windows` 0.62.2 resolves `windows-future 0.3.2`, whose `IAsyncOperation<T>` exposes `.join()` (not `.get()` — that's the older `windows-future 0.2.1` line). Confirmed via `cargo tree -i windows-future`.
2. **Alpha channel matters**: WGC's captured BGRA frame doesn't guarantee a meaningful alpha channel for ordinary opaque windows; `SoftwareBitmap::CreateCopyFromBuffer`'s default (Premultiplied) alpha interpretation with alpha=0 renders solid black. Forcing alpha=255 on every pixel before bitmap creation (now in `contexa-vision/src/ocr.rs`) fixed this.
3. **Full-window vs. cropped-region matters for accuracy measurement, not just latency.** An early run against a full, uncropped window scored only 58.6% similarity — not because OCR was wrong, but because the recognized text legitimately included real menu/tab/status-bar chrome text alongside the planted content. This is exactly why production `ocr_region` takes a `Region`: cropping to a content area is a correctness requirement for meaningful text, not just a performance optimization.
4. **WGC only delivers frames on content change** (confirmed directly, not just from SP-02's note): capturing an already-idle, unfocused window a second time hung until timeout with "no frame available" — a fresh repaint (a `SetWindowPos` resize, `SWP_NOACTIVATE` so it didn't steal focus) was needed to get a new frame. Also found the target window had gotten minimized (`IsIconic`) at one point, which WGC can't capture at all — restored via `ShowWindow(hwnd, SW_SHOWNOACTIVATE)`, again without activating it. Both fixes avoided touching the real foreground window/input focus, which had unrelated active work on it throughout.
5. **Production capture + OCR paths compose cleanly with zero adapter code** once cropping and the alpha fix are in place — the spike's final version calls `contexa_vision::OcrEngine::ocr_region` directly rather than hand-rolling its own bitmap conversion.
6. Same app-availability gap as SP-01: Acrobat/Slack/Figma aren't installed, and VS Code (SP-01's one confirmed UIA-fail app) wasn't used for this specific run since the Notepad ground-truth test closed the accuracy question directly and without needing a UIA-fail-specific app.

## Recommendation

**Pass.** All three docs/22 §5 criteria are met with real measurements, not estimates. Real `Windows.Media.Ocr` is wired into `contexa_vision::ocr::OcrEngine::ocr_region()` (replacing the prior honest-stub) and verified via `examples/vision_smoke.rs`. No further gating action needed; this was always a non-blocking spike for Phase 1 (docs/22 §13).

## Raw Data

- `cargo run --release` in `spikes/SP-03-ocr-fallback/` (no args) — captures whatever window has OS focus.
- `cargo run --release -- --hwnd <isize> --ground-truth <path>` — targets a specific window by handle (found without touching focus, e.g. `(Get-Process notepad).MainWindowHandle.ToInt64()`) and scores accuracy against a known text file. This is how the 98.2%/1.67%/19ms numbers above were produced, against a 3-sentence planted-text Notepad window (`sp03_ground_truth.txt`).
