//! `Embedder` + `EmbeddingPipeline` — `docs/07_Memory_Engine.md` §6.3.
//!
//! `ADR-0006` default only (`fastembed`/`all-MiniLM-L6-v2`, 384-dim); the
//! Ollama/`OpenAI` quality-mode embedding variants the spec sketches as an
//! `EmbeddingProvider` enum are out of scope for this pass — nothing in the
//! current roadmap needs them yet, and a two-thirds-unimplemented enum is
//! worse than a concrete type with a documented extension point.

use std::sync::{Arc, Mutex};

use fastembed::{EmbeddingModel as FastembedModel, InitOptions, TextEmbedding};

use contexa_core::{ContexaError, Result};
use contexa_db::{MemoryChunk, MemoryRepository};

pub const EMBEDDING_MODEL_NAME: &str = "all-MiniLM-L6-v2";
const DEFAULT_BATCH_SIZE: usize = 10;

/// Wraps a loaded fastembed model behind an `Arc<Mutex<_>>` so it can be
/// shared (one model load, not one per consumer) between `EmbeddingPipeline`
/// and `SemanticSearch`, and moved into `spawn_blocking` for the actual
/// (synchronous, CPU-bound) ONNX inference call.
#[derive(Clone)]
pub struct Embedder(Arc<Mutex<TextEmbedding>>);

impl Embedder {
    /// # Errors
    /// Returns an error if the fastembed model can't be loaded (e.g. a
    /// first-run download failure).
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(InitOptions::new(FastembedModel::AllMiniLML6V2))
            .map_err(|e| ContexaError::Conversion(e.to_string()))?;
        Ok(Self(Arc::new(Mutex::new(model))))
    }

    /// # Errors
    /// Returns an error if the embedding model fails or the blocking task
    /// panics.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model = Arc::clone(&self.0);
        tokio::task::spawn_blocking(move || {
            let mut model = model.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            model
                .embed(texts, None)
                .map_err(|e| ContexaError::Conversion(e.to_string()))
        })
        .await
        .map_err(|e| ContexaError::TaskJoin(e.to_string()))?
    }

    /// # Errors
    /// Returns an error if the embedding model fails or returns no vector.
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut vectors = self.embed_batch(vec![text.to_string()]).await?;
        vectors
            .pop()
            .ok_or_else(|| ContexaError::Conversion("fastembed returned no vector for query".to_string()))
    }
}

/// Batches chunks and flushes them (embed + persist) once `batch_size` is
/// reached, or on an explicit/periodic `flush()` (see `engine.rs`'s
/// `docs/07` §6.3 `flush_interval` timer).
pub struct EmbeddingPipeline {
    repo: Arc<dyn MemoryRepository>,
    embedder: Embedder,
    batch_size: usize,
    queue: Mutex<Vec<MemoryChunk>>,
}

impl EmbeddingPipeline {
    #[must_use]
    pub fn new(repo: Arc<dyn MemoryRepository>, embedder: Embedder) -> Self {
        Self {
            repo,
            embedder,
            batch_size: DEFAULT_BATCH_SIZE,
            queue: Mutex::new(Vec::new()),
        }
    }

    /// Enqueues `chunk`, flushing once `batch_size` is reached.
    ///
    /// # Errors
    /// Returns an error if a triggered flush's embedding or DB write fails.
    pub async fn enqueue(&self, chunk: MemoryChunk) -> Result<()> {
        let should_flush = {
            let mut queue = self.queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            queue.push(chunk);
            queue.len() >= self.batch_size
        };
        if should_flush {
            self.flush().await?;
        }
        Ok(())
    }

    /// Embeds and persists whatever is currently queued, even if under
    /// `batch_size` — used by the periodic flush timer so nothing waits
    /// indefinitely for the queue to fill.
    ///
    /// # Errors
    /// Returns an error if embedding or the DB write fails. Already-queued
    /// items are drained regardless (not re-queued on failure) — see the
    /// module doc for the accepted at-most-once tradeoff.
    pub async fn flush(&self) -> Result<()> {
        let batch = {
            let mut queue = self.queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *queue)
        };
        if batch.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = batch.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed_batch(texts).await?;

        for (chunk, embedding) in batch.into_iter().zip(embeddings) {
            self.repo.insert_chunk(&chunk).await?;
            self.repo
                .insert_embedding(&chunk.id.to_string(), &embedding, EMBEDDING_MODEL_NAME)
                .await?;
        }
        Ok(())
    }
}
