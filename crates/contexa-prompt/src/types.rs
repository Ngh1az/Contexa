//! `docs/10_Prompt_Builder.md` §9-10 types.

use contexa_core::{ContextSnapshot, UserRequest};
use contexa_db::{ScoredChunk, TimelineEvent};
use contexa_llm::Message;
use contexa_vision::OcrResult;

pub struct PromptInput {
    pub request: UserRequest,
    pub context: ContextSnapshot,
    pub memory: Vec<ScoredChunk>,
    pub search: Option<SearchResults>,
    pub timeline: Option<Vec<TimelineEvent>>,
    pub ocr: Option<OcrResult>,
}

/// Built from `contexa_search::SearchResponse.results` — carries title/url
/// alongside the snippet so `SearchFormatter` can render citations per
/// docs/09 §12, not just a flat snippet list.
pub struct SearchResults {
    pub items: Vec<SearchResultItem>,
}

pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub struct AssembledPrompt {
    pub system: String,
    pub messages: Vec<Message>,
    pub token_count: usize,
    pub sources: Vec<SourceRef>,
    pub truncated: bool,
}

pub struct SourceRef {
    pub source_type: SourceType,
    pub id: String,
    pub label: String,
    pub token_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Context,
    Selection,
    Memory,
    Search,
    Timeline,
    Ocr,
}
