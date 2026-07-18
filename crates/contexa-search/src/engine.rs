//! `ContexaSearchEngine` — `docs/09_Search_Engine.md` §7, wiring
//! `PrivacyGate` → `QueryFormulator` → `RateLimiter` → `SearchCache` →
//! `SearchAdapter`. Excludes the Content Fetcher / Local Semantic Ranker
//! stages (ADR-0014 — still Proposed, not Accepted); results come back in
//! whatever order the provider returns them, with a placeholder rank-based
//! `relevance_score`.

use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::Semaphore;

use contexa_core::{ContexaError, ContextSnapshot, Result};

use crate::adapter::SearchAdapter;
use crate::cache::SearchCache;
use crate::duckduckgo::DuckDuckGoAdapter;
use crate::privacy_gate::PrivacyGate;
use crate::query_formulator::QueryFormulator;
use crate::rate_limiter::RateLimiter;
use crate::types::{SearchResponse, WebSearchOptions};

// docs/09 §10.1: "Max concurrent searches | 2"
const MAX_CONCURRENT_SEARCHES: usize = 2;
const CACHE_CAPACITY: usize = 100;
const CACHE_TTL: Duration = Duration::from_secs(3600);

#[async_trait]
pub trait SearchEngine: Send + Sync {
    /// # Errors
    /// Returns `ContexaError::SearchDisabled` if disabled, `RateLimited` if
    /// over the call-rate limits, or the provider's error if the request fails.
    async fn search(&self, query: &str, context: &ContextSnapshot) -> Result<SearchResponse>;
    fn is_enabled(&self) -> bool;
    /// # Errors
    /// Currently infallible; `Result` return kept to match docs/09 §7's
    /// trait signature (a future provider could validate config at swap time).
    fn set_provider(&self, provider: Box<dyn SearchAdapter>) -> Result<()>;
}

pub struct ContexaSearchEngine {
    gate: PrivacyGate,
    formulator: QueryFormulator,
    rate_limiter: RateLimiter,
    cache: SearchCache,
    provider: RwLock<Arc<dyn SearchAdapter>>,
    concurrency: Semaphore,
}

impl ContexaSearchEngine {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            gate: PrivacyGate::new(enabled),
            formulator: QueryFormulator,
            rate_limiter: RateLimiter::default(),
            cache: SearchCache::new(CACHE_CAPACITY, CACHE_TTL),
            provider: RwLock::new(Arc::new(DuckDuckGoAdapter::default())),
            concurrency: Semaphore::new(MAX_CONCURRENT_SEARCHES),
        }
    }
}

impl Default for ContexaSearchEngine {
    fn default() -> Self {
        Self::new(false)
    }
}

#[async_trait]
impl SearchEngine for ContexaSearchEngine {
    async fn search(&self, query: &str, context: &ContextSnapshot) -> Result<SearchResponse> {
        self.gate.check()?;
        self.rate_limiter.check()?;
        let _permit = self
            .concurrency
            .acquire()
            .await
            .map_err(|_| ContexaError::Conversion("search concurrency semaphore closed".to_string()))?;

        let formulated = self.formulator.formulate(query, context);
        let start = Instant::now();

        if let Some(cached) = self.cache.get(&formulated) {
            let provider_name = provider_name(&self.provider);
            return Ok(SearchResponse {
                results: cached,
                query_used: formulated,
                provider: provider_name,
                cached: true,
                latency_ms: elapsed_ms(start),
            });
        }

        // Clone the `Arc` out and drop the lock guard before the `.await` —
        // holding a `std::sync::RwLock` guard across an await point would
        // block `set_provider` for the whole network round-trip.
        let provider = Arc::clone(&self.provider.read().unwrap_or_else(PoisonError::into_inner));
        let opts = WebSearchOptions {
            max_results: provider.max_results(),
            ..WebSearchOptions::default()
        };
        let mut results = provider.search(&formulated, opts).await?;

        for (i, result) in results.iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let rank_penalty = i as f32 * 0.1;
            result.relevance_score = (1.0 - rank_penalty).max(0.0);
        }

        self.cache.put(formulated.clone(), results.clone());

        Ok(SearchResponse {
            results,
            query_used: formulated,
            provider: provider.provider_name().to_string(),
            cached: false,
            latency_ms: elapsed_ms(start),
        })
    }

    fn is_enabled(&self) -> bool {
        self.gate.is_allowed()
    }

    fn set_provider(&self, provider: Box<dyn SearchAdapter>) -> Result<()> {
        *self.provider.write().unwrap_or_else(PoisonError::into_inner) = Arc::from(provider);
        Ok(())
    }
}

fn provider_name(provider: &RwLock<Arc<dyn SearchAdapter>>) -> String {
    provider
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .provider_name()
        .to_string()
}

fn elapsed_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::float_cmp)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::Utc;
    use contexa_core::CaptureMethod;
    use uuid::Uuid;

    use crate::types::SearchResult;

    use super::*;

    struct CountingAdapter {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl SearchAdapter for CountingAdapter {
        async fn search(&self, query: &str, _opts: WebSearchOptions) -> Result<Vec<SearchResult>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![SearchResult {
                title: format!("result for {query}"),
                snippet: "snippet".to_string(),
                url: "https://example.com".to_string(),
                published_date: None,
                relevance_score: 0.0,
            }])
        }
        fn provider_name(&self) -> &'static str {
            "counting"
        }
        fn max_results(&self) -> usize {
            5
        }
    }

    fn bare_context() -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: "t".to_string(),
            process_name: "p".to_string(),
            process_id: 1,
            hwnd: None,
            url: None,
            document_path: None,
            visible_text: None,
            selected_text: None,
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    #[tokio::test]
    async fn disabled_by_default_returns_search_disabled() {
        let engine = ContexaSearchEngine::default();
        let result = engine.search("rust", &bare_context()).await;
        assert!(matches!(result, Err(ContexaError::SearchDisabled)));
    }

    #[tokio::test]
    async fn enabled_search_uses_the_configured_provider_and_caches() {
        let engine = ContexaSearchEngine::new(true);
        engine
            .set_provider(Box::new(CountingAdapter {
                calls: AtomicUsize::new(0),
            }))
            .expect("set_provider");

        let first = engine
            .search("rust ownership", &bare_context())
            .await
            .expect("first search");
        assert!(!first.cached);
        assert_eq!(first.results[0].relevance_score, 1.0);

        let second = engine
            .search("rust ownership", &bare_context())
            .await
            .expect("second search");
        assert!(second.cached, "second identical query should hit the cache");
    }
}
