//! Region Hasher — `docs/05_Vision_Engine.md` §5.4. Skips UIA/OCR for regions
//! whose hash hasn't changed since the last check.

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct RegionHashCache {
    regions: HashMap<(isize, u32, u32), u64>,
}

impl RegionHashCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the cached hash for `(hwnd, row, col)` and returns `true` if it
    /// differs from what was previously cached (or nothing was cached yet).
    pub fn has_changed(&mut self, hwnd: isize, row: u32, col: u32, hash: u64) -> bool {
        let key = (hwnd, row, col);
        let changed = self.regions.get(&key) != Some(&hash);
        self.regions.insert(key, hash);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_check_is_a_change() {
        let mut cache = RegionHashCache::new();
        assert!(cache.has_changed(1, 0, 0, 42));
    }

    #[test]
    fn same_hash_is_not_a_change() {
        let mut cache = RegionHashCache::new();
        assert!(cache.has_changed(1, 0, 0, 42));
        assert!(!cache.has_changed(1, 0, 0, 42));
    }

    #[test]
    fn different_hash_is_a_change() {
        let mut cache = RegionHashCache::new();
        assert!(cache.has_changed(1, 0, 0, 42));
        assert!(cache.has_changed(1, 0, 0, 99));
    }

    #[test]
    fn different_region_is_independent() {
        let mut cache = RegionHashCache::new();
        assert!(cache.has_changed(1, 0, 0, 42));
        assert!(cache.has_changed(1, 0, 1, 42));
    }
}
