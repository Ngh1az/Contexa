//! Memory Engine — timeline, embeddings, semantic search — see `docs/07_Memory_Engine.md`

mod chunking;
mod deduplicator;
mod embedding;
mod engine;
mod retention_purger;
mod semantic_search;
mod timeline_builder;
mod types;
mod working_memory;

pub use embedding::{Embedder, EmbeddingPipeline, EMBEDDING_MODEL_NAME};
pub use engine::{ContexaMemoryEngine, MemoryEngine};
pub use retention_purger::RetentionPurger;
pub use semantic_search::SemanticSearch;
pub use timeline_builder::TimelineBuilder;
pub use types::SearchOptions;
pub use working_memory::WorkingMemory;
pub use deduplicator::Deduplicator;
