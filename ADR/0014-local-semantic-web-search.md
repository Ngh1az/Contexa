# ADR-0014: Local Semantic Ranking for Web Search (No Cloud AI Search API)

**Status:** Proposed
**Date:** 2026-07-15
**Deciders:** Architecture Team

---

## Context

Users want Exa-like semantic/neural web search — results ranked by meaning, not just keyword match, ideally citation-ready for LLM prompts. Options considered for reaching that quality bar:

- **Exa** — Purpose-built neural search API. Free tier exists but usage-capped; paid plans start at $99/mo. Cloud-hosted; queries and context leave the device.
- **Tavily** — AI-native search API, already evaluated and rejected as default in [ADR-0011](./0011-duckduckgo-default-search.md). Free tier (~1,000 credits/mo) available. Cloud-hosted.
- **Jina AI (s.jina.ai)** — Free tier (~100 req/min), LLM-optimized search + reader. Cloud-hosted, no self-host option.
- **SearXNG** — Open-source, self-hosted meta-search aggregator. Zero API fee, but requires the user to run and maintain an instance (VPS or local), which is friction ADR-0011 already ruled out for the default path.
- **Brave Search API** — Recommended opt-in provider per ADR-0011; **retired its perpetual free tier in Feb 2026** (now $5/mo credit). Does not affect this ADR's decision but weakens Brave's "free power-user option" framing from ADR-0011 and should be revisited separately.
- **NotebookLM** — Not applicable: Google has no public API for it; not an integration target.

All cloud options above send the query (and in some cases fetched page content) to a third party, which conflicts with Contexa's local-first / privacy-by-design positioning ([16_Security_Privacy.md](../docs/16_Security_Privacy.md)) unless the user explicitly opts in and supplies their own key — the same model already used for Brave/SerpAPI.

Separately, Contexa already has the two building blocks a neural search needs, both already accepted:
- Local embeddings via `fastembed` (384-dim, in-process, zero extra cost) — [ADR-0006](./0006-embedding-model.md)
- Local cross-encoder reranking — [ADR-0012](./0012-local-reranking.md), currently scoped to memory-chunk reranking in the prompt-build path only

## Decision

Add a **Local Semantic Ranker** stage to the Search Engine pipeline, positioned after the Provider Adapter and before results reach the Orchestrator:

1. **Retrieval** stays on the existing default: DuckDuckGo (free, zero-config, per ADR-0011). No change to the provider layer.
2. **Content Fetcher** retrieves the full text of each result URL (respecting robots.txt; capped concurrency; timeout-bounded).
3. **Local Semantic Ranker** embeds the query and fetched content with the same `fastembed` model already loaded for Memory Engine (ADR-0006 default, no new dependency), then reranks with the same local cross-encoder class introduced in ADR-0012, reused here rather than duplicated.
4. Top-k reranked results are returned to the Orchestrator in place of raw provider order.

This becomes Contexa's free, local, privacy-preserving analog to Exa's semantic search, without adopting a cloud AI search API as a dependency.

Cloud AI search APIs (Tavily, Jina, Exa) remain **possible future opt-in adapters** behind the existing `SearchAdapter` trait — same pattern as Brave/SerpAPI — but are **not implemented now** and are **not the default**. Their free tiers are noted for later reference only.

## Rationale

| Factor | Local Semantic Ranker (this ADR) | Cloud AI Search API (Exa/Tavily/Jina) | SearXNG self-host |
|--------|-----------------------------------|----------------------------------------|--------------------|
| Cost | Free | Free tier, then paid | Free (VPS cost only) |
| Privacy | Query + content never leave device | Query (and sometimes content) sent to third party | Query leaves device to SearXNG instance, not to Contexa vendor |
| New dependencies | None (reuses ADR-0006 + ADR-0012 stack) | New API client + credential vault entry | New self-hosted service to operate |
| Onboarding friction | None — works once search is enabled | API key signup required | User must deploy/maintain instance |
| Result freshness/coverage | Bounded by DuckDuckGo's index | Bounded by provider's own index (often broader) | Bounded by aggregated engines |
| Consistent with ADR-0011 default philosophy | Yes | No (reintroduces key-management friction ADR-0011 avoided) | No (reintroduces hosting friction) |

## Consequences

**Positive:**
- Delivers semantic-quality web search without a new cloud dependency or recurring cost
- Reuses two already-accepted architecture decisions instead of introducing a third search stack
- Keeps the door open for Tavily/Jina/Exa as opt-in adapters later without re-architecting `SearchAdapter`

**Negative:**
- Adds latency: fetch + embed + rerank per query, on top of the existing DuckDuckGo round-trip — needs its own budget (see Performance below)
- Quality ceiling is bounded by what DuckDuckGo surfaces in the first place; this reranks, it does not re-crawl the web like Exa does
- Content Fetcher introduces a new failure mode (page fetch timeout/blocked) that must degrade gracefully to unranked provider order, matching the "reranking is an enhancement, never a dependency" principle from ADR-0012

## Performance

| Stage | Budget |
|-------|--------|
| Content fetch (per result, parallelized) | < 800 ms |
| Embed query + N results | < 300 ms (reuses ADR-0006 batch path) |
| Rerank | < 150 ms (same budget as ADR-0012) |
| Total added latency (cache miss) | < 1.5 s, on top of existing < 3 s search budget in [09_Search_Engine.md](../docs/09_Search_Engine.md) |
| Fallback on timeout/fetch failure | Return provider order unranked — never block on this stage |

## Configuration

```json
{
  "search": {
    "enabled": false,
    "default_provider": "duckduckgo",
    "semantic_rank": {
      "enabled": true,
      "fetch_timeout_ms": 800,
      "fetch_concurrency": 3,
      "rerank_candidates": 10,
      "rerank_keep": 5
    }
  }
}
```

## References

- [09_Search_Engine.md](../docs/09_Search_Engine.md)
- [ADR-0006](./0006-embedding-model.md) — fastembed default embedding stack
- [ADR-0011](./0011-duckduckgo-default-search.md) — DuckDuckGo default, Brave opt-in
- [ADR-0012](./0012-local-reranking.md) — local cross-encoder reranking (memory chunks)
- [16_Security_Privacy.md](../docs/16_Security_Privacy.md)
