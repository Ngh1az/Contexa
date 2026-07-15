# SP-04: sqlite-vec Search at Scale

**Date:** 2026-07-08  
**Owner:**  
**Status:** Pass

## Summary
Full-scale run (50K × 384-dim, 100 queries) meets latency and DB size targets once the benchmark uses vec0’s KNN query form (`MATCH ... ORDER BY distance`). Insert performance is also well within expectations.

## Results

| Metric | Target | Actual | Pass? |
|--------|--------|--------|-------|
| Search p50 | < 100 ms | 51 ms (full: 50K vectors, 100 queries) | ✅ |
| Search p95 | < 200 ms | 57 ms (full: 50K vectors, 100 queries) | ✅ |
| Insert batch (10 vectors) | < 100 ms | p50=2 ms, p95=2 ms | ✅ |
| DB size (50K × 384-dim) | < 200 MB | 75.09 MB | ✅ |

## Observations
- Prebuilt release `vec0.dll` failed to load (“specified module could not be found”); rebuilding `vec0.dll` with MSVC resolved it.
- Insert performance looks good at 50K scale (~0.8s total).
- Measuring commit-per-10 inserts increases total insert time (~11.4s), but confirms batch latency is well under the 100ms criterion.
- Using `MATCH ... ORDER BY distance LIMIT k` is critical; `ORDER BY vec_distance_cosine(...)` behaves like a full scan at this scale.

## Recommendation
- Proceed with sqlite-vec (vec0) as planned for the 384-dim default path.
- Follow up: add a small insert-batch benchmark for “10 vectors < 100ms” (criterion not yet measured in this spike).

## Raw Data
- Command:
  - Smoke:
    - `cargo run --release -- --vectors 1000 --dims 384 --queries 5 --topk 10`
  - Full:
    - `cargo run --release -- --vectors 50000 --dims 384 --queries 100 --topk 10`
- Output (smoke):
  - Insert total: 14 ms
  - Query latency ms: p50=8, p95=9, p99=9
  - Query mean ms: 8.40
  - DB: `sp04.sqlite3` (local)
  - sqlite-vec extension: `D:\Contexa\vendor\sqlite-vec\vec0.dll`
- Output (full):
  - Insert total: 11449 ms
  - Insert batch(10) ms: p50=2, p95=2
  - Query latency ms: p50=51, p95=57, p99=58
  - Query mean ms: 52.35
  - DB size: 75.09 MB
- DB file path + size:
  - Smoke: `sp04.sqlite3` (~0 MB)
  - Full: `sp04.sqlite3` (75.09 MB)

