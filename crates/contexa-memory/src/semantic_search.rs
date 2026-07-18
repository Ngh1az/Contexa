//! `SemanticSearch` — `docs/07_Memory_Engine.md` §6.4.
//!
//! Returns `contexa_db::ScoredChunk` as-is (a flat `{id, content, timestamp,
//! application, distance}`, lower `distance` = better) rather than the
//! spec's `ScoredChunk { chunk: MemoryChunk, score: f32 }` sketch — that
//! type is already built and tested in `contexa-db`; inventing a second
//! shape here would just mean converting between them for no benefit.

use std::sync::Arc;

use contexa_db::{MemoryRepository, ScoredChunk};

use contexa_core::Result;

use crate::embedding::Embedder;
use crate::types::SearchOptions;

pub struct SemanticSearch {
    repo: Arc<dyn MemoryRepository>,
    embedder: Embedder,
}

impl SemanticSearch {
    #[must_use]
    pub fn new(repo: Arc<dyn MemoryRepository>, embedder: Embedder) -> Self {
        Self { repo, embedder }
    }

    /// # Errors
    /// Returns an error if embedding the query or the DB search fails.
    pub async fn search(&self, query: &str, opts: &SearchOptions) -> Result<Vec<ScoredChunk>> {
        let query_vector = self.embedder.embed_one(query).await?;
        // `embeddings`/`embeddings_768` are declared `distance_metric=cosine`
        // (migrations/V1__initial_schema.sql), so distance is cosine distance
        // (`1.0 - cosine_similarity`, range [0, 2]) — `min_score` -> `max_distance`
        // via `1.0 - score` matches docs/07 §9's cosine-similarity model exactly.
        // Cosine distance ranges [0, 2] (`1.0 - cosine_similarity`), not
        // [0, 1] — a `min_score` of 0 (or negative) must still allow
        // `max_distance` up to 2.0, not clamp it away.
        let max_distance = (1.0 - opts.min_score).clamp(0.0, 2.0);
        let mut results = self
            .repo
            .search_similar(&query_vector, opts.limit, max_distance)
            .await?;

        // `search_similar` doesn't support pushing time/application filters
        // into the query — post-filtering in Rust is a known limitation
        // (may return fewer than `opts.limit` results), not a correctness bug.
        if let Some(range) = &opts.time_range {
            results.retain(|r| r.timestamp >= range.start && r.timestamp <= range.end);
        }
        if let Some(app) = &opts.application_filter {
            results.retain(|r| &r.application == app);
        }

        Ok(results)
    }
}
