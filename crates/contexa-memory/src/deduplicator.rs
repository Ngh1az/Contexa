//! `Deduplicator` — `docs/07_Memory_Engine.md` §6.5: skip re-processing
//! content already seen recently, via a SHA-256 content hash in an LRU set.

use std::num::NonZeroUsize;

use lru::LruCache;
use sha2::{Digest, Sha256};

// Not spec'd; large enough to cover several minutes of context updates
// without letting the cache grow unbounded.
const DEFAULT_CAPACITY: usize = 1000;

pub struct Deduplicator {
    recent_hashes: LruCache<String, ()>,
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl Deduplicator {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN);
        Self {
            recent_hashes: LruCache::new(capacity),
        }
    }

    /// Returns `true` (and does not record it again) if `content` was seen
    /// recently; otherwise records it and returns `false`.
    pub fn is_duplicate(&mut self, content: &str) -> bool {
        let hash = content_hash(content);
        if self.recent_hashes.contains(&hash) {
            return true;
        }
        self.recent_hashes.put(hash, ());
        false
    }
}

/// Also used to populate `MemoryChunk::content_hash` (the DB's
/// `idx_memory_hash` unique index), not just this module's own dedup check.
#[must_use]
pub fn content_hash(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sighting_is_not_a_duplicate() {
        let mut dd = Deduplicator::default();
        assert!(!dd.is_duplicate("hello"));
    }

    #[test]
    fn repeated_content_is_a_duplicate() {
        let mut dd = Deduplicator::default();
        assert!(!dd.is_duplicate("hello"));
        assert!(dd.is_duplicate("hello"));
    }

    #[test]
    fn different_content_is_not_a_duplicate() {
        let mut dd = Deduplicator::default();
        assert!(!dd.is_duplicate("hello"));
        assert!(!dd.is_duplicate("world"));
    }

    #[test]
    fn evicts_least_recently_seen_once_over_capacity() {
        let mut dd = Deduplicator::new(1);
        assert!(!dd.is_duplicate("a"));
        assert!(!dd.is_duplicate("b")); // evicts "a"
        assert!(!dd.is_duplicate("a")); // "a" was evicted, so this is fresh
    }

    #[test]
    fn content_hash_is_deterministic_and_hex() {
        let a = content_hash("same input");
        let b = content_hash("same input");
        assert_eq!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
