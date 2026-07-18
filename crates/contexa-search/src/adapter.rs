//! `SearchAdapter` — `docs/09_Search_Engine.md` §5.3.

use async_trait::async_trait;

use contexa_core::Result;

use crate::types::{SearchResult, WebSearchOptions};

#[async_trait]
pub trait SearchAdapter: Send + Sync {
    /// # Errors
    /// Returns an error if the provider can't be reached; degrades to an
    /// empty `Vec` (not an error) if results parse to nothing.
    async fn search(&self, query: &str, opts: WebSearchOptions) -> Result<Vec<SearchResult>>;
    fn provider_name(&self) -> &'static str;
    fn max_results(&self) -> usize;
}
