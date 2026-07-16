//! Plugin Registry + Sandbox — `docs/18_Plugin_System.md` §7, §8.
//!
//! Enrichers are stored as `Arc<dyn ContextEnricher>` rather than docs/18's
//! `Box<dyn ContextEnricher>`: the sandbox needs to move an owned, cloneable
//! handle onto a detached `'static` thread per execution (see
//! `PluginSandbox` doc comment for why), and `Arc` is what makes that
//! possible without the registry giving up its own copy.

use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use contexa_core::ContextSnapshot;

use crate::enricher::{ContextEnricher, PluginInfo};

// docs/18 §8.1: "Timeout | 20ms per enricher; kill on timeout"
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(20);

#[derive(Default)]
pub struct PluginRegistry {
    enrichers: Vec<Arc<dyn ContextEnricher>>,
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, enricher: Arc<dyn ContextEnricher>) {
        self.enrichers.push(enricher);
        self.enrichers.sort_by_key(|e| std::cmp::Reverse(e.priority()));
    }

    #[must_use]
    pub fn get_enrichers(&self, process_name: &str) -> Vec<Arc<dyn ContextEnricher>> {
        self.enrichers
            .iter()
            .filter(|e| e.matches(process_name))
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn list_all(&self) -> Vec<PluginInfo> {
        self.enrichers.iter().map(|e| e.info()).collect()
    }
}

/// Runs enrichers with a timeout and error isolation (docs/18 §8.1: "Error
/// isolation", "No nested calls"). docs/18's own pseudocode uses
/// `std::thread::scope(...).spawn(...).join()` and treats a `join()` error
/// as "timed out" — but a scoped thread has no timeout mechanism; `join()`
/// blocks until the thread finishes, so that branch is actually the *panic*
/// case. Real timeout requires abandoning a still-running thread, which
/// `thread::scope` can't do (it always joins before returning). So this
/// spawns a plain, non-scoped thread that reports its result over an
/// `mpsc` channel; on timeout the thread is left detached rather than
/// joined — its eventual `send` just fails silently once the receiver is
/// dropped.
pub struct PluginSandbox {
    timeout: Duration,
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl PluginSandbox {
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Enriches a clone of `snapshot` off-thread and, if it finishes within
    /// the timeout without error, writes the result back into `snapshot`.
    /// On failure or timeout, `snapshot` is left unchanged — a bad enricher
    /// never blocks or corrupts the pipeline.
    pub fn execute(&self, enricher: &Arc<dyn ContextEnricher>, snapshot: &mut ContextSnapshot) {
        let plugin_id = enricher.info().id;
        let (tx, rx) = mpsc::channel();
        let enricher = Arc::clone(enricher);
        let mut working = snapshot.clone();
        let spawned = thread::Builder::new()
            .name("contexa-enricher".to_string())
            .spawn(move || {
                let result = enricher.enrich(&mut working);
                let _ = tx.send(result.map(|()| working));
            });
        if spawned.is_err() {
            tracing::warn!(plugin = %plugin_id, "failed to spawn enricher thread");
            return;
        }
        match rx.recv_timeout(self.timeout) {
            Ok(Ok(result_snapshot)) => *snapshot = result_snapshot,
            Ok(Err(error)) => tracing::warn!(plugin = %plugin_id, %error, "enricher failed"),
            Err(_) => {
                tracing::warn!(plugin = %plugin_id, timeout_ms = self.timeout.as_millis(), "enricher timed out");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::{CaptureMethod, ContexaError, Result};
    use uuid::Uuid;

    use super::*;

    fn blank_snapshot() -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: "Window".to_string(),
            process_name: "test.exe".to_string(),
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

    fn plugin_info(id: &str) -> PluginInfo {
        PluginInfo {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            author: "test".to_string(),
            description: String::new(),
        }
    }

    struct AlwaysMatch {
        priority: u32,
    }
    impl ContextEnricher for AlwaysMatch {
        fn matches(&self, _process_name: &str) -> bool {
            true
        }
        fn enrich(&self, snapshot: &mut ContextSnapshot) -> Result<()> {
            snapshot.metadata.insert("marked".to_string(), "yes".to_string());
            Ok(())
        }
        fn priority(&self) -> u32 {
            self.priority
        }
        fn info(&self) -> PluginInfo {
            plugin_info("test.always")
        }
    }

    struct NeverMatch;
    impl ContextEnricher for NeverMatch {
        fn matches(&self, _process_name: &str) -> bool {
            false
        }
        fn enrich(&self, _snapshot: &mut ContextSnapshot) -> Result<()> {
            Ok(())
        }
        fn info(&self) -> PluginInfo {
            plugin_info("test.never")
        }
    }

    struct Failing;
    impl ContextEnricher for Failing {
        fn matches(&self, _process_name: &str) -> bool {
            true
        }
        fn enrich(&self, _snapshot: &mut ContextSnapshot) -> Result<()> {
            Err(ContexaError::Conversion("boom".to_string()))
        }
        fn info(&self) -> PluginInfo {
            plugin_info("test.failing")
        }
    }

    struct SlowEnricher;
    impl ContextEnricher for SlowEnricher {
        fn matches(&self, _process_name: &str) -> bool {
            true
        }
        fn enrich(&self, _snapshot: &mut ContextSnapshot) -> Result<()> {
            thread::sleep(Duration::from_millis(200));
            Ok(())
        }
        fn info(&self) -> PluginInfo {
            plugin_info("test.slow")
        }
    }

    #[test]
    fn registry_only_returns_matching_enrichers() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(AlwaysMatch { priority: 0 }));
        registry.register(Arc::new(NeverMatch));
        let matches = registry.get_enrichers("anything.exe");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].info().id, "test.always");
    }

    #[test]
    fn registry_sorts_by_priority_descending() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(AlwaysMatch { priority: 10 }));
        registry.register(Arc::new(AlwaysMatch { priority: 90 }));
        let ids: Vec<u32> = registry
            .get_enrichers("x.exe")
            .iter()
            .map(|e| e.priority())
            .collect();
        assert_eq!(ids, vec![90, 10]);
    }

    #[test]
    fn sandbox_applies_successful_enrichment() {
        let sandbox = PluginSandbox::new(Duration::from_millis(50));
        let mut snapshot = blank_snapshot();
        let enricher: Arc<dyn ContextEnricher> = Arc::new(AlwaysMatch { priority: 0 });
        sandbox.execute(&enricher, &mut snapshot);
        assert_eq!(snapshot.metadata.get("marked"), Some(&"yes".to_string()));
    }

    #[test]
    fn sandbox_leaves_snapshot_unchanged_on_failure() {
        let sandbox = PluginSandbox::new(Duration::from_millis(50));
        let mut snapshot = blank_snapshot();
        let enricher: Arc<dyn ContextEnricher> = Arc::new(Failing);
        sandbox.execute(&enricher, &mut snapshot);
        assert!(snapshot.metadata.is_empty());
    }

    #[test]
    fn sandbox_leaves_snapshot_unchanged_on_timeout() {
        let sandbox = PluginSandbox::new(Duration::from_millis(10));
        let mut snapshot = blank_snapshot();
        let enricher: Arc<dyn ContextEnricher> = Arc::new(SlowEnricher);
        sandbox.execute(&enricher, &mut snapshot);
        assert!(snapshot.metadata.is_empty());
    }
}
