//! `docs/07_Memory_Engine.md` §9 types not already covered by `contexa_db`'s
//! `ScoredChunk`/`MemoryStats` (reused as-is — see `semantic_search.rs`).

use contexa_db::TimeRange;

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub min_score: f32,
    pub time_range: Option<TimeRange>,
    pub application_filter: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.7,
            time_range: None,
            application_filter: None,
        }
    }
}
