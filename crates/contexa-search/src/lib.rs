//! Search Engine — pluggable web search adapters — see `docs/09_Search_Engine.md`

mod adapter;
mod cache;
mod duckduckgo;
mod engine;
mod privacy_gate;
mod query_formulator;
mod rate_limiter;
mod types;

pub use adapter::SearchAdapter;
pub use cache::SearchCache;
pub use duckduckgo::DuckDuckGoAdapter;
pub use engine::{ContexaSearchEngine, SearchEngine};
pub use privacy_gate::PrivacyGate;
pub use query_formulator::QueryFormulator;
pub use rate_limiter::RateLimiter;
pub use types::{CachedResults, SearchResponse, SearchResult, WebSearchOptions};
