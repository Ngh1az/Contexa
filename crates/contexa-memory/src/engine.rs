//! `ContexaMemoryEngine` — `docs/07_Memory_Engine.md` §8, wiring working
//! memory, timeline, dedup, embedding, semantic search, and retention purge
//! together.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use contexa_core::{ContextSnapshot, Result};
use contexa_db::{
    ContextRepository, MemoryChunk, MemoryRepository, MemoryStats, Pagination, ScoredChunk,
    TimeRange, TimelineEvent, TimelineRepository,
};

use crate::chunking::{chunk_text, estimate_tokens};
use crate::deduplicator::{content_hash, Deduplicator};
use crate::embedding::{Embedder, EmbeddingPipeline};
use crate::retention_purger::RetentionPurger;
use crate::semantic_search::SemanticSearch;
use crate::timeline_builder::TimelineBuilder;
use crate::types::SearchOptions;
use crate::working_memory::WorkingMemory;

const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
// docs/07 §8's `get_timeline` takes only a `TimeRange`, no pagination param.
// `TimelineRepository::get_range` needs one — this is the page size until a
// paginated `MemoryEngine` API is actually requested.
const DEFAULT_TIMELINE_PAGE_SIZE: u32 = 200;

#[async_trait]
pub trait MemoryEngine: Send + Sync {
    /// # Errors
    /// Returns an error if persisting the snapshot, timeline event, or
    /// memory chunk fails.
    async fn ingest(&self, snapshot: &ContextSnapshot) -> Result<()>;
    /// # Errors
    /// Returns an error if embedding the query or the DB search fails.
    async fn search(&self, query: &str, opts: SearchOptions) -> Result<Vec<ScoredChunk>>;
    /// # Errors
    /// Returns an error if the DB query fails.
    async fn get_timeline(&self, range: TimeRange) -> Result<Vec<TimelineEvent>>;
    async fn get_working_memory(&self) -> Vec<ContextSnapshot>;
    /// # Errors
    /// Returns an error if the delete fails.
    async fn delete_chunk(&self, id: &str) -> Result<()>;
    /// # Errors
    /// Returns an error if the delete fails.
    async fn delete_all(&self) -> Result<u64>;
    /// # Errors
    /// Returns an error if the stats query fails.
    async fn get_stats(&self) -> Result<MemoryStats>;
}

pub struct ContexaMemoryEngine {
    working_memory: Mutex<WorkingMemory>,
    timeline_builder: Mutex<TimelineBuilder>,
    deduplicator: Mutex<Deduplicator>,
    embedding: Arc<EmbeddingPipeline>,
    semantic_search: SemanticSearch,
    context_repo: Arc<dyn ContextRepository>,
    memory_repo: Arc<dyn MemoryRepository>,
    timeline_repo: Arc<dyn TimelineRepository>,
}

impl ContexaMemoryEngine {
    /// # Errors
    /// Returns an error if the fastembed model can't be loaded.
    pub fn new(
        context_repo: Arc<dyn ContextRepository>,
        memory_repo: Arc<dyn MemoryRepository>,
        timeline_repo: Arc<dyn TimelineRepository>,
    ) -> Result<Self> {
        let embedder = Embedder::new()?;
        let embedding = Arc::new(EmbeddingPipeline::new(
            Arc::clone(&memory_repo),
            embedder.clone(),
        ));
        let semantic_search = SemanticSearch::new(Arc::clone(&memory_repo), embedder);

        // Periodic flush per docs/07 §6.3 (`flush_interval`, default 5s) — a
        // detached background task, same lifecycle simplification already
        // used by `contexa-vision`'s capture thread / `contexa-context`'s
        // consumer thread (runs until process exit; no shutdown handle yet).
        let flush_target = Arc::clone(&embedding);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(FLUSH_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(e) = flush_target.flush().await {
                    tracing::warn!(error = %e, "periodic embedding flush failed");
                }
            }
        });

        Ok(Self {
            working_memory: Mutex::new(WorkingMemory::default()),
            timeline_builder: Mutex::new(TimelineBuilder::default()),
            deduplicator: Mutex::new(Deduplicator::default()),
            embedding,
            semantic_search,
            context_repo,
            memory_repo,
            timeline_repo,
        })
    }

    /// # Errors
    /// Returns an error if the underlying purge query fails.
    pub async fn purge(&self, retention_days: u32) -> Result<contexa_db::PurgeStats> {
        RetentionPurger::new(retention_days)
            .purge(self.memory_repo.as_ref())
            .await
    }
}

// docs/07 §7.1's flow diagram checks "significance" before dedup/embedding
// but never defines the rule; visible or selected text present is the
// simplest reasonable bar — an app-switch with no readable content isn't
// worth a memory chunk.
fn is_significant(snapshot: &ContextSnapshot) -> bool {
    snapshot
        .visible_text
        .as_deref()
        .is_some_and(|t| !t.trim().is_empty())
        || snapshot
            .selected_text
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty())
}

#[async_trait]
impl MemoryEngine for ContexaMemoryEngine {
    async fn ingest(&self, snapshot: &ContextSnapshot) -> Result<()> {
        {
            let mut wm = self
                .working_memory
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            wm.push(snapshot.clone());
        }

        // Session memory: every context update is persisted (docs/07 §5
        // tiers), regardless of significance — only chunk/embedding creation
        // below is gated by it. Must happen before the timeline event insert:
        // `timeline_events.context_id` has a FK on `context_snapshots(id)`.
        self.context_repo.insert_snapshot(snapshot).await?;

        let timeline_event = {
            let mut tb = self
                .timeline_builder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            tb.process_context_change(snapshot)
        };
        if let Some(event) = timeline_event {
            self.timeline_repo.insert_event(&event).await?;
        }

        if !is_significant(snapshot) {
            return Ok(());
        }

        let Some(text) = snapshot
            .visible_text
            .as_deref()
            .filter(|t| !t.trim().is_empty())
        else {
            return Ok(());
        };

        let is_duplicate = {
            let mut dedup = self
                .deduplicator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            dedup.is_duplicate(text)
        };
        if is_duplicate {
            return Ok(());
        }

        for piece in chunk_text(text) {
            let chunk = MemoryChunk {
                id: Uuid::new_v4(),
                context_id: Some(snapshot.id),
                content_hash: content_hash(&piece),
                content: piece.clone(),
                timestamp: snapshot.timestamp,
                application: snapshot.process_name.clone(),
                metadata: HashMap::new(),
                token_count: i64::try_from(estimate_tokens(&piece)).unwrap_or(i64::MAX),
            };
            self.embedding.enqueue(chunk).await?;
        }

        Ok(())
    }

    async fn search(&self, query: &str, opts: SearchOptions) -> Result<Vec<ScoredChunk>> {
        self.semantic_search.search(query, &opts).await
    }

    async fn get_timeline(&self, range: TimeRange) -> Result<Vec<TimelineEvent>> {
        let page = self
            .timeline_repo
            .get_range(
                range,
                Pagination {
                    limit: DEFAULT_TIMELINE_PAGE_SIZE,
                    offset: 0,
                },
            )
            .await?;
        Ok(page.items)
    }

    async fn get_working_memory(&self) -> Vec<ContextSnapshot> {
        self.working_memory
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_all()
    }

    async fn delete_chunk(&self, id: &str) -> Result<()> {
        self.memory_repo.delete_chunk(id).await
    }

    async fn delete_all(&self) -> Result<u64> {
        self.memory_repo.delete_all().await
    }

    async fn get_stats(&self) -> Result<MemoryStats> {
        self.memory_repo.get_stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn significance_requires_visible_or_selected_text() {
        let mut snapshot = bare_snapshot();
        assert!(!is_significant(&snapshot));

        snapshot.visible_text = Some("   ".to_string());
        assert!(!is_significant(&snapshot), "whitespace-only isn't significant");

        snapshot.visible_text = Some("real content".to_string());
        assert!(is_significant(&snapshot));
    }

    fn bare_snapshot() -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
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
            capture_method: contexa_core::CaptureMethod::Uia,
        }
    }
}
