# ADR-0006: Local Embedding Model Selection

**Status:** Accepted (amended 2026-07-07)  
**Date:** 2026-07-06  
**Deciders:** Architecture Team  
**Validates:** SP-05 in [22_Technical_Spike_Plan.md](../docs/22_Technical_Spike_Plan.md)

---

## Context

The Memory Engine requires an embedding model to generate vector representations of context chunks for semantic search. Options include cloud APIs (OpenAI), local models via Ollama, and local ONNX models.

Requirements:
- Must work offline (local-first principle)
- Batch embed 10 chunks in < 2 seconds (default path)
- Model size < 500 MB
- MRR@10 > 0.7 on work-context search queries
- Vector dimensions must be supported by sqlite-vec

## Decision

Use **`fastembed` + all-MiniLM-L6-v2** (384 dimensions) as the **default** embedding provider (zero extra process), with **`nomic-embed-text`** (768 dimensions) via **Ollama** as an **opt-in quality mode** when Ollama is installed.

Cloud embedding (OpenAI) remains an optional override in Settings.

## Alternatives Considered

| Model | Dims | Size | Latency (10 chunks) | MRR@10 | Verdict |
|-------|------|------|---------------------|--------|---------|
| all-MiniLM-L6-v2 (ONNX) | 384 | 80 MB | ~0.3s | 0.71 | **Selected (default)** |
| nomic-embed-text (Ollama) | 768 | 274 MB | ~1.5s | 0.78 | **Selected (quality opt-in)** |
| text-embedding-3-small (OpenAI) | 1536 | API | ~0.5s | 0.82 | Rejected as default (cloud dependency) |
| bge-small-en-v1.5 (ONNX) | 384 | 130 MB | ~0.4s | 0.74 | Considered; MiniLM preferred for size |

## Rationale

1. **Zero-config default** — fastembed runs in-process; no Ollama required for semantic search on day one
2. **Lower resource footprint** — 384-dim vectors use half the storage of 768-dim (~1.5 KB vs ~3 KB per vector)
3. **Faster batch embed** — ~0.3s vs ~1.5s; better for real-time memory pipeline
4. **Quality upgrade path** — users with Ollama (already recommended for LLM per ADR-0007) can enable nomic-embed-text in Settings
5. MRR 0.71 meets the > 0.7 threshold; quality mode reaches 0.78 for recall-heavy users

## Schema Impact

```sql
-- Default: 384 dimensions (fastembed)
CREATE VIRTUAL TABLE embeddings USING vec0(
    chunk_id    TEXT PRIMARY KEY,
    embedding   FLOAT[384]
);

-- Quality mode: nomic-embed-text (768-dim)
CREATE VIRTUAL TABLE embeddings_768 USING vec0(
    chunk_id    TEXT PRIMARY KEY,
    embedding   FLOAT[768]
);
```

`embedding_meta.model` and `user_settings.embedding_provider` track which table is active. Switching providers requires re-indexing (background job with progress UI).

## Consequences

**Positive:**
- Fully offline semantic search without Ollama
- No API costs for default embeddings
- Single-process architecture for typical users
- Quality tier available when Ollama is present

**Negative:**
- Default search quality slightly below nomic (0.71 vs 0.78 MRR)
- Re-indexing required when user switches embedding models
- Two vector tables to maintain during transition

## Amendment History

| Date | Change |
|------|--------|
| 2026-07-06 | Initial: nomic-embed-text default |
| 2026-07-07 | Amended: fastembed default; nomic quality opt-in (docs v1.3 audit) |

## References

- [07_Memory_Engine.md](../docs/07_Memory_Engine.md)
- [04_Database_Design.md](../docs/04_Database_Design.md)
- [nomic-embed-text](https://ollama.com/library/nomic-embed-text)
- [fastembed-rs](https://github.com/Anush008/fastembed-rs)
