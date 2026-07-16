//! Context Cache — `docs/06_Context_Engine.md` §5.4. Thread-safe in-memory
//! cache for instant context access. Uses a capacity-bounded `VecDeque`
//! instead of docs/06's `LruCache`: `get_recent` only ever filters by
//! timestamp, never looks up by key, so no key-based eviction is needed and
//! `lru` isn't a workspace dependency.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono::Utc;

use contexa_core::ContextSnapshot;

const RECENT_CAPACITY: usize = 100; // docs/06 §5.4: "Last 100 snapshots"

pub struct ContextCache {
    current: Arc<RwLock<Option<ContextSnapshot>>>,
    recent: Arc<RwLock<VecDeque<ContextSnapshot>>>,
}

impl Default for ContextCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: Arc::new(RwLock::new(None)),
            recent: Arc::new(RwLock::new(VecDeque::with_capacity(RECENT_CAPACITY))),
        }
    }

    /// `None` until the first snapshot has been captured — returning a fake
    /// placeholder would misrepresent "nothing observed yet" as real context.
    #[must_use]
    pub fn get_current(&self) -> Option<ContextSnapshot> {
        self.current
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn update(&self, snapshot: ContextSnapshot) {
        {
            let mut recent = self
                .recent
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if recent.len() >= RECENT_CAPACITY {
                recent.pop_front();
            }
            recent.push_back(snapshot.clone());
        }
        *self
            .current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(snapshot);
    }

    #[must_use]
    pub fn get_recent(&self, duration: Duration) -> Vec<ContextSnapshot> {
        let cutoff =
            Utc::now() - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::zero());
        self.recent
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|s| s.timestamp > cutoff)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use contexa_core::CaptureMethod;

    use super::*;

    fn snapshot(timestamp: DateTime<Utc>) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp,
            window_title: "Notepad".to_string(),
            process_name: "notepad.exe".to_string(),
            process_id: 1,
            hwnd: Some(1),
            url: None,
            document_path: None,
            visible_text: None,
            selected_text: None,
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    #[test]
    fn empty_cache_has_no_current() {
        assert!(ContextCache::new().get_current().is_none());
    }

    #[test]
    fn update_sets_current_and_recent() {
        let cache = ContextCache::new();
        let s = snapshot(Utc::now());
        let id = s.id;
        cache.update(s);
        assert_eq!(cache.get_current().map(|s| s.id), Some(id));
        assert_eq!(cache.get_recent(Duration::from_secs(60)).len(), 1);
    }

    #[test]
    fn get_recent_filters_by_cutoff() {
        let cache = ContextCache::new();
        cache.update(snapshot(Utc::now() - chrono::Duration::hours(2)));
        cache.update(snapshot(Utc::now()));
        let recent = cache.get_recent(Duration::from_secs(300));
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn recent_is_bounded_to_capacity() {
        let cache = ContextCache::new();
        for _ in 0..(RECENT_CAPACITY + 10) {
            cache.update(snapshot(Utc::now()));
        }
        assert_eq!(
            cache.get_recent(Duration::from_secs(3600)).len(),
            RECENT_CAPACITY
        );
    }
}
