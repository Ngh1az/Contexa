//! `ContextEnricher` plugin trait — `docs/18_Plugin_System.md` §5.1.

use contexa_core::{ContextSnapshot, Result};

pub trait ContextEnricher: Send + Sync {
    /// Returns true if this enricher handles the given process.
    fn matches(&self, process_name: &str) -> bool;

    /// Enriches the context snapshot with app-specific data.
    ///
    /// # Errors
    /// Returns an error if extraction fails; the pipeline logs and continues
    /// rather than failing the whole snapshot (docs/18 §8.1).
    fn enrich(&self, snapshot: &mut ContextSnapshot) -> Result<()>;

    /// Execution priority — higher runs first.
    fn priority(&self) -> u32 {
        0
    }

    fn info(&self) -> PluginInfo;
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}
