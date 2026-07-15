# ADR-0009: SQLCipher for At-Rest Database Encryption

**Status:** Accepted  
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
- 40–60% write overhead; search latency +40%
- Key loss = permanent data loss — must warn users clearly
- Must verify `sqlite-vec` loads correctly after `PRAGMA key` — **gate: SP-09**
- Migration complexity for existing users upgrading to encrypted DB
- If SP-09 fails: fallback to **Windows DPAPI column-level encryption** for `visible_text` and `memory_chunks.content` (ADR-0009 scope reduced)

## References

- [ADR/0010-rusqlite-database-access.md](./0010-rusqlite-database-access.md)
- [22_Technical_Spike_Plan.md](../docs/22_Technical_Spike_Plan.md) — SP-09
- [04_Database_Design.md](../docs/04_Database_Design.md) §16
- [16_Security_Privacy.md](../docs/16_Security_Privacy.md) §15
- [SQLCipher](https://www.zetetic.net/sqlcipher/)
