# ADR-0010: rusqlite Database Access Layer

**Status:** Accepted  
**Date:** 2026-07-07  
**Deciders:** Architecture Team  
**Supersedes:** Partial guidance in ADR-0003 (sqlx mention)

---

## Context

Contexa requires SQLite access with:

1. **sqlite-vec** extension loading at connection init
2. **SQLCipher** support for Pro tier (ADR-0009)
3. WAL mode with single-writer queue
4. Schema migrations on startup

Two Rust options were documented without a final decision:

- **`sqlx`** — Async, compile-time query checks; weak sqlite-vec and SQLCipher extension support
- **`rusqlite`** — Sync API; mature extension loading; `bundled-sqlcipher` feature

## Decision

Use **`rusqlite`** as the sole database access layer, with **`refinery`** for migrations.

Async engine code calls the DB via `tokio::task::spawn_blocking` on a dedicated connection pool (one write connection + N read connections).

## Alternatives Considered

| Option | Verdict |
|--------|---------|
| `sqlx` only | Rejected — extension loading and SQLCipher are awkward |
| `sqlx` + `rusqlite` hybrid | Rejected — two SQLite stacks increase complexity |
| `rusqlite` + `refinery` | **Selected** — one stack; proven extension support |
| `diesel` | Rejected — ORM overhead unnecessary for this schema |

## Rationale

1. **sqlite-vec** must be loaded via `conn.load_extension()` or static link — `rusqlite` supports this directly
2. **SQLCipher** uses `PRAGMA key` before any query — must run before extension load order is validated (SP-09)
3. **refinery** works with plain SQL migration files; no async runtime coupling
4. Contexa's DB workload is bursty writes + indexed reads — `spawn_blocking` overhead is negligible vs network I/O

## Implementation Sketch

```rust
// contexa-db crate
pub struct Database {
    write_conn: Mutex<Connection>,       // single writer
    read_pool: Arc<Vec<Connection>>,     // round-robin readers (WAL)
}

impl Database {
    pub fn open(path: &Path, key: Option<&str>) -> Result<Self> {
        let conn = Connection::open(path)?;
        if let Some(k) = key {
            conn.execute_batch(&format!("PRAGMA key = '{}';", k))?;
        }
        conn.load_extension("vec0", None)?; // or static link
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        // ...
    }
}

// Async wrapper
pub async fn search_similar(db: Arc<Database>, vector: &[f32]) -> Result<Vec<ScoredChunk>> {
    tokio::task::spawn_blocking(move || db.search_similar_sync(vector)).await?
}
```

## Consequences

**Positive:**
- Single, consistent DB stack for vec search, encryption, and migrations
- Simpler spike validation (SP-04, SP-09)
- `refinery` migrations are plain SQL — easy to review

**Negative:**
- No compile-time SQL checking (mitigate with integration tests)
- `spawn_blocking` required for all DB calls from async context
- Team must not introduce `sqlx` for SQLite in parallel

## References

- [ADR/0003-sqlite-local-storage.md](./0003-sqlite-local-storage.md)
- [ADR/0009-sqlcipher-encryption.md](./0009-sqlcipher-encryption.md)
- [04_Database_Design.md](../docs/04_Database_Design.md)
- [22_Technical_Spike_Plan.md](../docs/22_Technical_Spike_Plan.md) — SP-04, SP-09
