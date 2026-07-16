//! `ContextEngine` trait + `ContexaContextEngine` — `docs/06_Context_Engine.md`
//! §7. Wires: `SnapshotAssembler` -> `PluginRegistry`/`PluginSandbox` ->
//! language detector -> `ChangeDetector` -> `ContextCache` -> broadcast.
//!
//! `process_vision_result` is a plain, synchronous, `&self` method — it does
//! not own a thread that drains `contexa_vision`'s `Receiver<VisionResult>`.
//! `ContexaVisionEngine::new()` hands back that raw receiver expecting a
//! caller to drive it (see its doc comment); the composition root (not this
//! crate) is responsible for looping `rx.recv()` and calling this method.

use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use tokio::sync::broadcast;

use contexa_core::{ContextSnapshot, Result};
use contexa_vision::VisionResult;

use crate::assembler::SnapshotAssembler;
use crate::cache::ContextCache;
use crate::change_detector::ChangeDetector;
use crate::enricher::ContextEnricher;
use crate::enrichers::{ChromiumEnricher, VsCodeEnricher};
use crate::language::detect_language;
use crate::registry::{PluginRegistry, PluginSandbox};

// Buffer for lagging subscribers (docs/06 §7 `subscribe`); no spec'd size —
// picked to comfortably absorb a burst of window switches between reads.
const BROADCAST_CAPACITY: usize = 32;

pub trait ContextEngine: Send + Sync {
    /// `None` until the first snapshot has been captured.
    fn get_current(&self) -> Option<ContextSnapshot>;
    fn get_recent(&self, duration: Duration) -> Vec<ContextSnapshot>;
    /// # Errors
    /// Returns an error if a registered enricher's sandboxed thread can't be
    /// spawned. Reserved for future failure modes described in docs/06 §7.
    fn process_vision_result(&self, result: VisionResult) -> Result<Option<ContextSnapshot>>;
    fn subscribe(&self) -> broadcast::Receiver<ContextSnapshot>;
    /// Always `None` for now — Selection Tracker (docs/06 §5.5) is deferred.
    fn get_selection(&self) -> Option<String>;
    fn register_enricher(&self, enricher: Arc<dyn ContextEnricher>);
}

pub struct ContexaContextEngine {
    assembler: SnapshotAssembler,
    registry: RwLock<PluginRegistry>,
    sandbox: PluginSandbox,
    change_detector: Mutex<ChangeDetector>,
    cache: ContextCache,
    events_tx: broadcast::Sender<ContextSnapshot>,
}

impl ContexaContextEngine {
    #[must_use]
    pub fn new() -> Self {
        let (events_tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            assembler: SnapshotAssembler,
            registry: RwLock::new(PluginRegistry::new()),
            sandbox: PluginSandbox::default(),
            change_detector: Mutex::new(ChangeDetector::new()),
            cache: ContextCache::new(),
            events_tx,
        }
    }

    /// Registers the built-in enrichers (Chrome, Edge, VS Code). docs/18
    /// §10.2 best practice: "Register enrichers at startup; avoid runtime
    /// registration in production."
    #[must_use]
    pub fn with_builtin_enrichers() -> Self {
        let engine = Self::new();
        engine.register_enricher(Arc::new(ChromiumEnricher::chrome()));
        engine.register_enricher(Arc::new(ChromiumEnricher::edge()));
        engine.register_enricher(Arc::new(VsCodeEnricher));
        engine
    }
}

impl Default for ContexaContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextEngine for ContexaContextEngine {
    fn get_current(&self) -> Option<ContextSnapshot> {
        self.cache.get_current()
    }

    fn get_recent(&self, duration: Duration) -> Vec<ContextSnapshot> {
        self.cache.get_recent(duration)
    }

    fn process_vision_result(&self, result: VisionResult) -> Result<Option<ContextSnapshot>> {
        let mut snapshot = self.assembler.assemble(result);

        let enrichers = self
            .registry
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_enrichers(&snapshot.process_name);
        for enricher in &enrichers {
            self.sandbox.execute(enricher, &mut snapshot);
        }

        if let Some(text) = &snapshot.visible_text {
            snapshot.language = detect_language(text);
        }

        let changed = self
            .change_detector
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .has_changed(&snapshot);
        if !changed {
            return Ok(None);
        }

        self.cache.update(snapshot.clone());
        let _ = self.events_tx.send(snapshot.clone()); // Ok: no subscribers yet is not an error
        Ok(Some(snapshot))
    }

    fn subscribe(&self) -> broadcast::Receiver<ContextSnapshot> {
        self.events_tx.subscribe()
    }

    fn get_selection(&self) -> Option<String> {
        None
    }

    fn register_enricher(&self, enricher: Arc<dyn ContextEnricher>) {
        self.registry
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .register(enricher);
    }
}

#[cfg(test)]
mod tests {
    use contexa_core::CaptureMethod;

    use super::*;
    use crate::enricher::PluginInfo;

    fn vision_result(hwnd: isize, title: &str, process_name: &str) -> VisionResult {
        VisionResult {
            hwnd,
            window_title: title.to_string(),
            process_name: process_name.to_string(),
            process_id: 1,
            frame_hash: [0; 4],
            changed_regions: Vec::new(),
            uia_result: None,
            ocr_result: None,
            capture_method: None,
            timestamp: chrono::Utc::now(),
        }
    }

    struct TagEnricher;
    impl ContextEnricher for TagEnricher {
        fn matches(&self, process_name: &str) -> bool {
            process_name.eq_ignore_ascii_case("tagged.exe")
        }
        fn enrich(&self, snapshot: &mut ContextSnapshot) -> Result<()> {
            snapshot
                .metadata
                .insert("tagged".to_string(), "yes".to_string());
            Ok(())
        }
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "test.tag".to_string(),
                name: "Tag".to_string(),
                version: "0.1.0".to_string(),
                author: "test".to_string(),
                description: String::new(),
            }
        }
    }

    #[test]
    fn first_vision_result_emits_a_snapshot_and_updates_cache() {
        let engine = ContexaContextEngine::new();
        let result = engine.process_vision_result(vision_result(1, "Window", "app.exe"));
        assert!(matches!(result, Ok(Some(_))));
        assert!(engine.get_current().is_some());
    }

    #[test]
    fn identical_second_result_emits_nothing() {
        let engine = ContexaContextEngine::new();
        let _ = engine.process_vision_result(vision_result(1, "Window", "app.exe"));
        let second = engine.process_vision_result(vision_result(1, "Window", "app.exe"));
        assert!(matches!(second, Ok(None)));
    }

    #[test]
    fn registered_enricher_runs_for_matching_process() {
        let engine = ContexaContextEngine::new();
        engine.register_enricher(Arc::new(TagEnricher));
        let result = engine.process_vision_result(vision_result(1, "Window", "tagged.exe"));
        let Ok(Some(snapshot)) = result else {
            panic!("expected a snapshot");
        };
        assert_eq!(
            snapshot.metadata.get("tagged").map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn subscriber_receives_broadcast_snapshot() {
        let engine = ContexaContextEngine::new();
        let mut rx = engine.subscribe();
        let _ = engine.process_vision_result(vision_result(1, "Window", "app.exe"));
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn get_selection_is_none_for_now() {
        assert_eq!(ContexaContextEngine::new().get_selection(), None);
    }

    #[test]
    fn capture_method_field_is_carried_through() {
        let engine = ContexaContextEngine::new();
        let result = engine.process_vision_result(vision_result(1, "Window", "app.exe"));
        let Ok(Some(snapshot)) = result else {
            panic!("expected a snapshot");
        };
        assert_eq!(snapshot.capture_method, CaptureMethod::Uia);
    }
}
