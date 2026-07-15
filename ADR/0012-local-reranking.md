# ADR-0012: Local Reranking Model for Semantic Search

**Status:** Proposed  
**Date:** 2026-07-14  
**Deciders:** Architecture Team  
**Target:** v1.1

---

## Context

Semantic search retrieves memory chunks via sqlite-vec KNN over 384-dim embeddings (ADR-0006, ADR-0003). Bi-encoder retrieval is fast but coarse: the top-k by cosine distance often includes topically-adjacent but irrelevant chunks, which then consume prompt token budget ([10_Prompt_Builder.md](../docs/10_Prompt_Builder.md)).

A reranking stage re-scores the retrieved candidates against the query with a cross-encoder, keeping only the best few for the prompt.

Options considered:

- **Cohere Rerank API** — Best quality; requires sending memory chunks (user screen content) to a cloud API
- **Local cross-encoder via fastembed** — e.g. `BAAI/bge-reranker-base` (ONNX); runs on the embedding stack already in the app
- **No reranking** — status quo; rely on cosine top-k only

## Decision

Add a **local reranking stage** in v1.1 using a cross-encoder model served by **fastembed's ONNX rerank API** (candidate: `bge-reranker-base`; final model selected by a spike mirroring SP-05).

- Rerank runs **only in the prompt-build path** (Orchestrator → Prompt Builder), not for timeline/UI search.
- Pipeline: sqlite-vec KNN top-20 → cross-encoder rerank → top-5 into prompt.
- Budget: 150 ms for the rerank stage. On timeout or model-load failure, **fall back to cosine order** — reranking is an enhancement, never a dependency.
- **Cloud rerankers are rejected**, not deferred: rerank input is raw memory chunk text (screen content), and sending it to a third-party API violates Privacy by Design regardless of user opt-in framing.

## Rationale

| Factor | Local cross-encoder | Cohere API | No rerank |
|--------|--------------------|-----------|-----------|
| Privacy (chunks leave device) | Never | Always | Never |
| Quality lift over cosine top-k | High | Highest | — |
| Offline operation | Yes | No | Yes |
| Added latency (top-20) | ~50–150 ms CPU | ~200–500 ms network | 0 |
| New dependencies | None (fastembed already ships ONNX) | API key + billing | — |

The existing search target (< 200 ms for 10K vectors) applies to retrieval; the rerank stage gets its own budget because it only runs when building a prompt, where total budget is dominated by LLM time-to-first-token (< 1 s).

## Consequences

**Positive:**
- Higher-relevance context in prompts with zero privacy cost
- Reuses the fastembed/ONNX stack — no new runtime
- Graceful degradation preserves current behavior

**Negative:**
- ~100–300 MB additional model download (lazy, on first enable) — mitigate with opt-in or on-demand fetch like the Ollama quality path (ADR-0006)
- ~50–150 ms added to prompt build on CPU — bounded by the 150 ms budget
- One more model to version and benchmark — add a rerank case to the Criterion suite

## Configuration

```json
{
  "search": {
    "rerank": {
      "enabled": true,
      "model": "bge-reranker-base",
      "candidates": 20,
      "keep": 5,
      "timeout_ms": 150
    }
  }
}
```

## References

- [ADR-0006](./0006-embedding-model.md) — fastembed default embedding stack
- [09_Search_Engine.md](../docs/09_Search_Engine.md)
- [10_Prompt_Builder.md](../docs/10_Prompt_Builder.md)
- [28_Tech_Expansion_Plan.md](../docs/28_Tech_Expansion_Plan.md)
