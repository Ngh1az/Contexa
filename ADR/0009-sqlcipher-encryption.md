# ADR-0009: SQLCipher for At-Rest Database Encryption

**Status:** Accepted (SP-09 validated 2026-07-18, one required config: `cache_size` tuning — see SP-09 update)  
**Date:** 2026-07-07  
**Deciders:** Architecture Team  
**Target:** v1.1 (Pro tier)

---

## Context

Contexa stores sensitive desktop context locally in SQLite: URLs, document text, code selections, timeline, and memory embeddings. OS file permissions protect against casual access but not disk theft or forensic recovery.

Users and Pro tier customers need encryption at rest without cloud dependency.

## Decision

Use **SQLCipher 4** to encrypt the entire `contexa.db` file. Feature is **opt-in**, available on **Pro tier** from v1.1.

- Encryption key derived from user passphrase (optional) + stored in Windows Credential Vault for auto-unlock
- `rusqlite` with `bundled-sqlcipher` feature
- Migration path: plain → encrypted copy → secure delete plain backup

## Alternatives Considered

| Option | Verdict |
|--------|---------|
| OS-level BitLocker only | Rejected — not Contexa's control; insufficient for Pro value prop |
| Field-level encryption | Rejected — complex; breaks sqlite-vec indexing |
| SQLCipher whole-DB | **Selected** — industry standard; transparent to queries |
| Encrypted container (VeraCrypt) | Rejected — poor UX |

## Consequences

**Positive:**
- Pro tier differentiation aligned with privacy positioning
- Full database protected including embeddings and timeline
- No cloud key escrow required

**Negative:**
- 40–60% write overhead
- Key loss = permanent data loss — must warn users clearly
- Must verify `sqlite-vec` loads correctly after `PRAGMA key` — **gate: SP-09**
- Migration complexity for existing users upgrading to encrypted DB
- **Requires `PRAGMA cache_size` tuned to the working set** (not SQLite's ~2MB default) — see SP-09
  update; without this, search latency regresses badly on scan-heavy `vec0` queries
- If SP-09 fails: fallback to **Windows DPAPI column-level encryption** for `visible_text` and `memory_chunks.content` (ADR-0009 scope reduced) — **not needed**, see SP-09 update

## SP-09 Update (2026-07-18)

SP-09 ran: `sqlite-vec` loads correctly after `PRAGMA key` and encrypted insert/search work
(compatibility criterion **passes**). Unlock-on-open stays `<1ms` at every scale tested, well under
the 100ms target.

**First pass** (SQLite's default `cache_size`, ~2MB): search p95 delta vs. plain DB was
**+254–300%** at 10K–50K vectors (target `< +50%`) — far worse than this ADR's +40% estimate.
Root cause: un-indexed `vec0` KNN is a full-table scan, and with a page cache far smaller than the
DB, every query re-evicts and re-reads pages, forcing SQLCipher to redo AES+HMAC decryption on each
one. The plain DB doesn't pay this because a SQLite cache miss just falls back to a cheap OS-cached
read; an encrypted cache miss is CPU-bound cipher work, every query.

**Second pass:** sizing `PRAGMA cache_size` to hold the vector table's working set (rather than
SQLite's default) eliminates the repeated decryption — each page is decrypted once and reused from
SQLite's in-process cache thereafter. Result: **encrypted search became 58–71% *faster* than plain**
at every scale tested (1K/10K/50K, 3 independent 50K runs), comfortably beating the `< +50%` target
in the other direction.

**Decision:** the SP-09 fail action is **not** invoked — the compatibility problem was a missing
production configuration, not an architectural incompatibility. Whole-DB SQLCipher remains in scope
for all tables including `vec_items`/embedding search, **conditional on `contexa-db` setting
`PRAGMA cache_size` (or `mmap_size`) to the expected working-set size at connection open**, not
leaving it at SQLite's ~2MB default. No DPAPI column-level fallback is needed. Does not block Phase 1
GA regardless — only the Pro-tier SQLCipher feature (v1.1). Full data:
[spikes/SP-09-sqlcipher/report.md](../spikes/SP-09-sqlcipher/report.md).

## References

- [ADR/0010-rusqlite-database-access.md](./0010-rusqlite-database-access.md)
- [22_Technical_Spike_Plan.md](../docs/22_Technical_Spike_Plan.md) — SP-09
- [04_Database_Design.md](../docs/04_Database_Design.md) §16
- [16_Security_Privacy.md](../docs/16_Security_Privacy.md) §15
- [SQLCipher](https://www.zetetic.net/sqlcipher/)
