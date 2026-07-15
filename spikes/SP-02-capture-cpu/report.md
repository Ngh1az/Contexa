# SP-02: Graphics Capture + Frame Diff CPU

**Date:** 2026-07-14  
**Owner:** —  
**Status:** Pass

## Summary

WGC capture loop with 16×16 average-hash frame differencing (sampled at ¼ resolution) stays far under every CPU target on a 32-core machine: 0.01–0.14% of machine (task-manager style) across 1/5/10 fps against a window with live streaming text. Memory 55 MB. A 30-minute soak (10 min per state) is appended below.

## Results (60 s per state)

| State | FPS | Target | CPU (of machine) | CPU (single-core) | Pass? |
|-------|-----|--------|------------------|-------------------|-------|
| Idle | 1 | < 1% | 0.01% | 0.44% | ✅ |
| Active | 5 | < 3% | 0.05% | 1.46% | ✅ |
| Interactive | 10 | < 5% | 0.14% | 4.40% | ✅ |
| Memory | — | < 100 MB | 55 MB | — | ✅ |

Frame diff behavior: 58/157/540 frames delivered per state; >5% hamming change threshold correctly separated changed vs unchanged frames (e.g. idle state: 6 changed, 51 skipped → production would skip UIA/OCR on the 51).

## Soak run (600 s per state, 30 min total)

| State | Frames | CPU (of machine) | CPU (single-core) | Mem |
|-------|--------|------------------|-------------------|-----|
| Idle 1 fps | 471 (58 changed / 412 skipped) | 0.02% | 0.55% | 55 MB |
| Active 5 fps | 0 (target window went static) | 0.00% | 0.09% | 56 MB |
| Interactive 10 fps | 0 (target window went static) | 0.01% | 0.25% | 42 MB |

The idle state is the strongest soak datapoint: 10 sustained minutes with real frame flow at 0.02% machine CPU. The active/interactive states lost their activity source mid-run (the window being captured stopped changing), degenerating into the zero-frame case — which itself confirms that a static window costs ~0 regardless of tick rate. The 60 s runs above cover 5/10 fps **with** frame flow; both datasets are far inside targets.

## Observations

- **Bug caught for Phase 1:** dropping `GraphicsCaptureSession` stops frame delivery silently — the session must be kept alive alongside the frame pool. First run produced 0 frames because of this.
- WGC only delivers frames when window content actually changes — an idle window costs effectively zero CPU regardless of tick rate. Capture-avoidance is built into the OS API.
- Hash cost is trivial: staging-texture copy + map dominates, not the hash arithmetic. No SIMD/GPU needed (confirms docs/28 disposition).
- On reference hardware (12 threads, i5-12400) the same single-core numbers would be ~0.04% / 0.12% / 0.37% of machine — still >10× inside targets.

## Recommendation

Proceed with WGC + free-threaded frame pool + cheap average-hash differencing for Phase 1. No GPU compute shader or SIMD justification exists at these numbers (docs/28 §4.8 triggers not fired).

## Raw Data

- `spikes/SP-02-capture-cpu/target/release/sp02-capture-cpu.exe [seconds_per_state]` — assert-based gate check built in.
