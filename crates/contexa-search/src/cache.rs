//! `SearchCache` — `docs/09_Search_Engine.md` §5.7. Caches the final result
//! set (not raw provider output) so a cache hit skips fetch entirely.

use std::num::NonZeroUsize;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use chrono::Utc;
use lru::LruCache;

use crate::types::{CachedResults, SearchResult};

pub struct SearchCache {
    cache: Mutex<LruCache<String, CachedResults>>,
    ttl: Duration,
}

impl SearchCache {
    #[must_use]
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN);
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            ttl,
        }
    }

    /// Returns `None` on a miss, or if the cached entry is past its TTL
    /// (which also evicts it).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<Vec<SearchResult>> {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let ttl = chrono::Duration::from_std(self.ttl).unwrap_or_default();
        let is_expired = cache
            .peek(key)
            .is_some_and(|entry| Utc::now() - entry.cached_at > ttl);
        if is_expired {
            cache.pop(key);
            return None;
        }
        cache.get(key).map(|entry| entry.results.clone())
    }

    pub fn put(&self, key: String, results: Vec<SearchResult>) {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.put(
            key,
            CachedResults {
                results,
                cached_at: Utc::now(),
            },
        );
    }
}

impl Default for SearchCache {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(3600))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_result() -> SearchResult {
        SearchResult {
            title: "t".to_string(),
            snippet: "s".to_string(),
            url: "https://example.com".to_string(),
            published_date: None,
            relevance_score: 1.0,
        }
    }

    #[test]
    fn miss_then_hit() {
        let cache = SearchCache::new(10, Duration::from_secs(3600));
        assert!(cache.get("q").is_none());
        cache.put("q".to_string(), vec![sample_result()]);
        let hit = cache.get("q").expect("should hit after put");
        assert_eq!(hit.len(), 1);
    }

    #[test]
    fn expired_entry_is_evicted_on_read() {
        let cache = SearchCache::new(10, Duration::from_millis(0));
        cache.put("q".to_string(), vec![sample_result()]);
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get("q").is_none());
    }
}
