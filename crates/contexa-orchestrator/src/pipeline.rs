//! `PipelineManager` — `docs/08_AI_Orchestrator.md` §6: gathers context,
//! memory, (optionally) OCR and timeline, builds the prompt, and calls the
//! LLM provider.
//!
//! Deviations from the spec's pseudocode, and why:
//! - Context + memory aren't fetched via `tokio::join!` — `ContextEngine::get_current`
//!   is a plain synchronous method (already built that way in `contexa-context`),
//!   so there's nothing to parallelize it with.
//! - `search` calls the real `contexa_search::SearchEngine`; on any failure
//!   (disabled, rate-limited, provider error) it falls back to `None` — the
//!   documented fallback path ("Search fails → proceed with local context
//!   only", docs/08 §13), now an actual fallback rather than a permanent stub.
//! - OCR defaults to `Region::whole_window()` — `ocr.rs::crop()` clamps to
//!   the actual captured frame, so a full-window OCR doesn't need the
//!   caller to know the window's real size or screen position up front.
//!   `trigger_ocr` still accepts a caller-supplied region for future
//!   selection-targeted OCR; `execute()` just doesn't have one to pass yet.

use std::sync::Arc;

use chrono::Utc;

use contexa_context::ContextEngine;
use contexa_core::{ContexaError, ExecutionPlan, Result, UserRequest};
use contexa_db::TimeRange;
use contexa_llm::{CompletionOptions, ProviderSelector, ResponseStream};
use contexa_memory::{MemoryEngine, SearchOptions};
use contexa_prompt::{PromptBuilder, PromptInput, SearchResultItem, SearchResults};
use contexa_search::SearchEngine;
use contexa_vision::{OcrResult, Region, VisionEngine};

// No request preference carries a recall date range yet — a week is a
// reasonable default recall window until one is added.
const DEFAULT_TIMELINE_LOOKBACK_DAYS: i64 = 7;

#[derive(Default)]
pub struct PipelineConfig {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

pub struct PipelineManager {
    context: Arc<dyn ContextEngine>,
    memory: Arc<dyn MemoryEngine>,
    vision: Arc<dyn VisionEngine>,
    search: Arc<dyn SearchEngine>,
    prompt_builder: Arc<dyn PromptBuilder>,
    provider: ProviderSelector,
    config: PipelineConfig,
}

impl PipelineManager {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryEngine>,
        vision: Arc<dyn VisionEngine>,
        search: Arc<dyn SearchEngine>,
        prompt_builder: Arc<dyn PromptBuilder>,
        provider: ProviderSelector,
        config: PipelineConfig,
    ) -> Self {
        Self {
            context,
            memory,
            vision,
            search,
            prompt_builder,
            provider,
            config,
        }
    }

    /// # Errors
    /// Returns an error if no context is available, a memory/timeline fetch
    /// fails, prompt assembly fails, or every configured LLM provider fails.
    pub async fn execute(&self, plan: ExecutionPlan, request: UserRequest) -> Result<ResponseStream> {
        let context = request
            .context_override
            .clone()
            .or_else(|| self.context.get_current())
            .ok_or(ContexaError::ContextUnavailable)?;

        let memory = if plan.need_memory {
            match request.query.as_deref().filter(|q| !q.trim().is_empty()) {
                Some(query) => self.memory.search(query, SearchOptions::default()).await?,
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };

        let ocr = if plan.need_ocr {
            self.trigger_ocr(Some(Region::whole_window())).await
        } else {
            None
        };

        let search = if plan.need_search {
            match request.query.as_deref().filter(|q| !q.trim().is_empty()) {
                Some(query) => self.run_search(query, &context).await,
                None => None,
            }
        } else {
            None
        };

        let timeline = if plan.need_timeline {
            let range = TimeRange {
                start: Utc::now() - chrono::Duration::days(DEFAULT_TIMELINE_LOOKBACK_DAYS),
                end: Utc::now(),
            };
            Some(self.memory.get_timeline(range).await?)
        } else {
            None
        };

        let prompt = self.prompt_builder.build(PromptInput {
            request,
            context,
            memory,
            search,
            timeline,
            ocr,
        })?;

        let opts = CompletionOptions {
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: true,
        };

        self.provider.complete(&prompt.messages, opts).await
    }

    /// Real OCR call, gated on a caller-supplied region — `None` skips OCR
    /// entirely (used when nothing needs it); `execute()` passes
    /// `Region::whole_window()` when `plan.need_ocr` is set.
    async fn trigger_ocr(&self, region: Option<Region>) -> Option<OcrResult> {
        let Some(region) = region else {
            tracing::debug!(
                "OCR requested but no capture region is wired yet; proceeding with UIA text only"
            );
            return None;
        };

        let vision = Arc::clone(&self.vision);
        let result = tokio::task::spawn_blocking(move || vision.ocr_region(&region)).await;

        match result {
            Ok(Ok(ocr)) => Some(ocr),
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "OCR failed; proceeding with UIA text only");
                None
            }
            Err(e) => {
                tracing::warn!(error = %e, "OCR task panicked; proceeding with UIA text only");
                None
            }
        }
    }

    /// Real web search, degrading to `None` on any failure — disabled,
    /// rate-limited, or a provider error all fall through to "proceed with
    /// local context only" (docs/08 §13) rather than failing the request.
    async fn run_search(
        &self,
        query: &str,
        context: &contexa_core::ContextSnapshot,
    ) -> Option<SearchResults> {
        match self.search.search(query, context).await {
            Ok(response) => Some(SearchResults {
                items: response
                    .results
                    .into_iter()
                    .map(|r| SearchResultItem {
                        title: r.title,
                        url: r.url,
                        snippet: r.snippet,
                    })
                    .collect(),
            }),
            Err(e) => {
                tracing::warn!(error = %e, "web search failed; proceeding with local context only");
                None
            }
        }
    }
}
