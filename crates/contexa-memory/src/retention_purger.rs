//! `RetentionPurger` — `docs/07_Memory_Engine.md` §6.6.

use chrono::Utc;

use contexa_core::Result;
use contexa_db::{MemoryRepository, PurgeStats};

const DEFAULT_RETENTION_DAYS: u32 = 90;

pub struct RetentionPurger {
    retention_days: u32,
}

impl Default for RetentionPurger {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION_DAYS)
    }
}

impl RetentionPurger {
    #[must_use]
    pub fn new(retention_days: u32) -> Self {
        Self { retention_days }
    }

    /// `MemoryRepository::purge_before` already VACUUMs unconditionally —
    /// not re-adding the spec's ">1000 rows" conditional on top of that.
    ///
    /// # Errors
    /// Returns an error if the underlying purge query fails.
    pub async fn purge(&self, repo: &dyn MemoryRepository) -> Result<PurgeStats> {
        let cutoff = Utc::now() - chrono::Duration::days(i64::from(self.retention_days));
        repo.purge_before(cutoff).await
    }
}
