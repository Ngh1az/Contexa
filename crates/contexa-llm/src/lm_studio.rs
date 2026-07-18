//! LM Studio adapter — local, OpenAI-compatible server (no API key). Thin
//! wrapper over `OpenAiProvider`'s wire format, which LM Studio implements
//! verbatim (lmstudio.ai/docs/app/api/endpoints/openai) — reuse rather than
//! re-implement the same request/response shapes.

use async_trait::async_trait;

use contexa_core::Result;

use crate::openai::OpenAiProvider;
use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream};

const DEFAULT_BASE_URL: &str = "http://localhost:1234/v1";
// Context window, not queryable — model-dependent, same conservative default
// as ollama.rs.
const DEFAULT_MAX_TOKENS: usize = 8192;

pub struct LmStudioProvider {
    inner: OpenAiProvider,
}

impl LmStudioProvider {
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            // LM Studio ignores the Authorization header; no key needed.
            inner: OpenAiProvider::new(String::new(), model).with_base_url(DEFAULT_BASE_URL),
        }
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self.inner.with_base_url(base_url);
        self
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream> {
        self.inner.complete(messages, opts).await
    }

    fn max_tokens(&self) -> usize {
        DEFAULT_MAX_TOKENS
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &'static str {
        "lm-studio"
    }
}
