# ADR-0011: DuckDuckGo as Default Search Provider

**Status:** Accepted (amended 2026-07-15)  
**Date:** 2026-07-07  
**Deciders:** Architecture Team

---

## Context

The Search Engine provides web search when desktop context is insufficient. Users expect search to work without API key setup on first use.

Options considered:

- **Brave Search API** — High quality; requires API key and account
- **DuckDuckGo** — No API key; HTML/lite API scraping
- **SerpAPI** — Google results; paid API key required
- **Tavily** — AI-native search; API key required

## Decision

Use **DuckDuckGo** as the **default** search provider (zero configuration). **Brave Search** is the recommended **opt-in** provider when the user supplies an API key (Pro tier feature).

Search remains **disabled by default** at the app level (privacy gate); when enabled, DuckDuckGo requires no additional setup.

## Rationale

| Factor | DuckDuckGo | Brave |
|--------|------------|-------|
| API key required | No | Yes |
| Cost | Free | Paid — perpetual free tier retired Feb 2026; now $5/mo credit only (see Amendment) |
| Onboarding friction | Minimal | User must sign up (and pay) |
| Privacy positioning | Aligns with local-first | Good; but key management |
| Rate limits | Informal; cache mitigates | Formal API quotas |
| Result quality | Adequate for dev context | Higher |

Contexa's search is a **fallback** when local memory lacks answers — adequate quality with zero config beats higher quality with setup friction for v1.0.

## Consequences

**Positive:**
- Search works immediately when user toggles it on
- No API key storage for default path
- Brave remains available for power users

**Negative:**
- DuckDuckGo HTML scraping may break if markup changes — mitigated by adapter tests and cache
- No official SLA — mitigated by provider fallback chain: DuckDuckGo → Brave (if key) → SerpAPI (if key)

## Configuration

```json
{
  "search": {
    "enabled": false,
    "default_provider": "duckduckgo",
    "providers": {
      "duckduckgo": { "enabled": true },
      "brave": { "enabled": false, "api_key_vault": "contexa-brave-key" }
    }
  }
}
```

## Amendment (2026-07-15): Brave free tier retired

Brave Search API retired its perpetual free tier in **February 2026**; it now grants a $5/month credit only, with paid plans beyond that. Brave was already scoped in this ADR as an **opt-in, key-required, Pro tier** provider — never the zero-config default — so **the decision (DuckDuckGo default) is unaffected**.

What changes: the framing of Brave as a "free power-user option" no longer holds — it is now a paid provider with no free path at all, which further widens the gap this ADR already identified between DuckDuckGo (always free) and Brave (always requires payment/signup). No action needed beyond updating user-facing docs/UI copy that may still describe Brave as "free with API key."

This does not affect the fallback chain (DuckDuckGo → Brave → SerpAPI, all opt-in beyond DuckDuckGo) or the Local Semantic Ranker decision in [ADR-0014](./0014-local-semantic-web-search.md), which reranks DuckDuckGo results locally rather than depending on any paid provider.

## Amendment History

| Date | Change |
|------|--------|
| 2026-07-07 | Initial: DuckDuckGo default, Brave opt-in |
| 2026-07-15 | Amended: noted Brave's Feb 2026 free-tier retirement; no change to the default decision |

## References

- [09_Search_Engine.md](../docs/09_Search_Engine.md)
- [16_Security_Privacy.md](../docs/16_Security_Privacy.md)
- [ADR-0014](./0014-local-semantic-web-search.md) — local semantic ranking over DuckDuckGo results
