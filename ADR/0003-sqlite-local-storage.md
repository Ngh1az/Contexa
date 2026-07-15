# ADR-0003: SQLite for Local-First Storage

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team

---

## Context

Contexa needs persistent storage for context snapshots, timeline events, memory chunks, vector embeddings, and user settings. The platform is local-first with no cloud database by default.

Options considered:
- **SQLite** — Embedded relational database
- **RocksDB** — Embedded key-value store
- **Sled** — Pure Rust embedded database
- **PostgreSQL** — External database server
- **JSON files** — Simple file-based storage

## Decision

Use **SQLite 3** with the **sqlite-vec** extension for vector similarity search.

## Rationale

| Factor | SQLite | RocksDB | Sled | PostgreSQL |
|--------|--------|---------|------|------------|
| Setup | Zero-config | Zero-config | Zero-config | Requires server |
| SQL queries | Full SQL | Key-value only | Key-value only | Full SQL |
| Vector search | sqlite-vec | Manual | Manual | pgvector |
| Ecosystem | Massive | Large | Small | Large |
| Rust support | rusqlite (see ADR-0010) | rust-rocksdb | sled crate | sqlx, tokio-postgres |
| WAL mode | Yes | N/A | N/A | Yes |
| Max DB size | 281 TB | Unlimited | Unlimited | Unlimited |
| Maturity | 20+ years | 10+ years | Unstable API | 25+ years |

SQLite is the standard for embedded local storage. The sqlite-vec extension adds vector search without requiring a separate vector database. WAL mode enables concurrent reads during writes, essential for the capture pipeline.

## Consequences

**Positive:**
- Zero configuration for users
- Single file database (`contexa.db`) — easy backup and portability
- Full SQL for timeline queries, retention purge, and analytics
- sqlite-vec provides cosine similarity search in-process
- Well-understood by developers; extensive tooling

**Negative:**
- Single-writer limitation (mitigated by write queue)
- May need partitioning strategy beyond 10 GB
- sqlite-vec is relatively new (monitor stability)
- No built-in encryption in v1.0 (SQLCipher opt-in Pro tier — see [ADR-0009](./0009-sqlcipher-encryption.md))

## Configuration

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;
PRAGMA mmap_size = 268435456;
```

## References

- [04_Database_Design.md](../docs/04_Database_Design.md)
- [sqlite-vec](https://github.com/asg017/sqlite-vec)
