//! Context Engine — snapshot assembly, enrichment, caching — see `docs/06_Context_Engine.md`

mod assembler;
mod cache;
mod change_detector;
mod engine;
mod enricher;
mod enrichers;
mod language;
mod registry;
mod selection;

pub use assembler::SnapshotAssembler;
pub use cache::ContextCache;
pub use change_detector::ChangeDetector;
pub use engine::{ContextEngine, ContexaContextEngine};
pub use enricher::{ContextEnricher, PluginInfo};
pub use enrichers::{ChromiumEnricher, VsCodeEnricher};
pub use language::detect_language;
pub use registry::{PluginRegistry, PluginSandbox};
pub use selection::{NoSelectionSource, SelectionSource, SelectionTracker};
