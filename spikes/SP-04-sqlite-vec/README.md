## SP-04 — sqlite-vec search at scale

**Spec:** `docs/22_Technical_Spike_Plan.md` (SP-04), `docs/04_Database_Design.md`, `docs/07_Memory_Engine.md`, `ADR/0010`

### Goal

Measure semantic search latency over **50K × 384-dim** vectors using:
- SQLite (WAL)
- `rusqlite`
- `sqlite-vec` (`vec0`)

### Deliverable

- `report.md` updated with p50/p95/p99 query latency + DB size
- Raw output from `cargo run --release` pasted/linked in the report

### How to run

1. Ensure you have Rust installed.
2. Provide a built `sqlite-vec` extension path via env var:
   - `SQLITE_VEC_PATH` = path to `vec0` extension file (`.dll` on Windows).
3. Run:
   - `cargo run --release -- --vectors 50000 --dims 384 --queries 100`

This spike intentionally keeps code minimal and self-contained.

