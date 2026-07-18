//! `docs/09_Search_Engine.md` §7-8 types.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub published_date: Option<String>,
    pub relevance_score: f32,
}

#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query_used: String,
    pub provider: String,
    pub cached: bool,
    pub latency_ms: u64,
}

#[derive(Debug, Clone)]
pub struct WebSearchOptions {
    pub max_results: usize,
    pub language: Option<String>,
    pub safe_search: bool,
}

impl Default for WebSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            language: None,
            safe_search: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedResults {
    pub results: Vec<SearchResult>,
    pub cached_at: DateTime<Utc>,
}
