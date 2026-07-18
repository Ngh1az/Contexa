//! `WorkingMemory` — `docs/07_Memory_Engine.md` §6.1: last-30-min in-memory
//! ring buffer.

use std::collections::VecDeque;
use std::time::Duration;

use chrono::Utc;
use contexa_core::ContextSnapshot;

const DEFAULT_MAX_SIZE: usize = 200;
const DEFAULT_MAX_AGE: Duration = Duration::from_secs(30 * 60);

pub struct WorkingMemory {
    buffer: VecDeque<ContextSnapshot>,
    max_size: usize,
    max_age: Duration,
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_SIZE, DEFAULT_MAX_AGE)
    }
}

impl WorkingMemory {
    #[must_use]
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self {
            buffer: VecDeque::new(),
            max_size,
            max_age,
        }
    }

    pub fn push(&mut self, snapshot: ContextSnapshot) {
        self.buffer.push_back(snapshot);
        while self.buffer.len() > self.max_size {
            self.buffer.pop_front();
        }
        self.evict_expired();
    }

    // Spec sketches this as `Vec<&ContextSnapshot>`, but callers here go
    // through a `Mutex` (see `engine.rs`) — a borrow tied to the guard can't
    // outlive the lock. Cloning out is the simplest fix; snapshots are small.
    #[must_use]
    pub fn get_all(&self) -> Vec<ContextSnapshot> {
        self.buffer.iter().cloned().collect()
    }

    fn evict_expired(&mut self) {
        let Ok(max_age) = chrono::Duration::from_std(self.max_age) else {
            return;
        };
        let cutoff = Utc::now() - max_age;
        while self.buffer.front().is_some_and(|s| s.timestamp < cutoff) {
            self.buffer.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use contexa_core::CaptureMethod;

    use super::*;

    fn snapshot_at(timestamp: chrono::DateTime<Utc>) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp,
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

    #[test]
    fn evicts_oldest_beyond_max_size() {
        let mut wm = WorkingMemory::new(2, Duration::from_secs(3600));
        wm.push(snapshot_at(Utc::now()));
        wm.push(snapshot_at(Utc::now()));
        wm.push(snapshot_at(Utc::now()));
        assert_eq!(wm.get_all().len(), 2);
    }

    #[test]
    fn evicts_snapshots_older_than_max_age() {
        let mut wm = WorkingMemory::new(200, Duration::from_secs(60));
        wm.push(snapshot_at(Utc::now() - chrono::Duration::hours(1)));
        wm.push(snapshot_at(Utc::now()));
        assert_eq!(wm.get_all().len(), 1);
    }
}
