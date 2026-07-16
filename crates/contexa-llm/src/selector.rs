//! `ProviderSelector` — `docs/08_AI_Orchestrator.md` §7. Tries the primary
//! provider; falls back to a secondary on failure (docs/08 §13 fallback chain).

use contexa_core::Result;

use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream};

pub struct ProviderSelector {
    primary: Box<dyn LlmProvider>,
    fallback: Option<Box<dyn LlmProvider>>,
}

impl ProviderSelector {
    #[must_use]
    pub fn new(primary: Box<dyn LlmProvider>) -> Self {
        Self {
            primary,
            fallback: None,
        }
    }

    #[must_use]
    pub fn with_fallback(mut self, fallback: Box<dyn LlmProvider>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// # Errors
    /// Returns the fallback provider's error if both providers fail (or the
    /// primary's error if no fallback is configured).
    pub async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream> {
        match self.primary.complete(messages, opts.clone()).await {
            Ok(stream) => Ok(stream),
            Err(e) => {
                let Some(fallback) = &self.fallback else {
                    return Err(e);
                };
                tracing::warn!(
                    primary = self.primary.provider_name(),
                    fallback = fallback.provider_name(),
                    error = %e,
                    "primary LLM provider failed; trying fallback"
                );
                fallback.complete(messages, opts).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use contexa_core::ContexaError;

    use super::*;

    struct AlwaysFails(&'static str);
    #[async_trait]
    impl LlmProvider for AlwaysFails {
        async fn complete(&self, _messages: &[Message], _opts: CompletionOptions) -> Result<ResponseStream> {
            Err(ContexaError::LlmProviderError {
                provider: self.0.to_string(),
                message: "simulated failure".to_string(),
            })
        }
        fn max_tokens(&self) -> usize {
            0
        }
        fn supports_streaming(&self) -> bool {
            false
        }
        fn provider_name(&self) -> &'static str {
            self.0
        }
    }

    struct AlwaysSucceeds(&'static str);
    #[async_trait]
    impl LlmProvider for AlwaysSucceeds {
        async fn complete(&self, _messages: &[Message], _opts: CompletionOptions) -> Result<ResponseStream> {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let _ = tx.send(Ok("ok".to_string()));
            Ok(rx)
        }
        fn max_tokens(&self) -> usize {
            1000
        }
        fn supports_streaming(&self) -> bool {
            true
        }
        fn provider_name(&self) -> &'static str {
            self.0
        }
    }

    #[tokio::test]
    async fn uses_primary_when_it_succeeds() {
        let selector = ProviderSelector::new(Box::new(AlwaysSucceeds("primary")));
        let result = selector.complete(&[], CompletionOptions::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn falls_back_when_primary_fails() {
        let selector = ProviderSelector::new(Box::new(AlwaysFails("primary")))
            .with_fallback(Box::new(AlwaysSucceeds("fallback")));
        let result = selector.complete(&[], CompletionOptions::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn propagates_error_when_no_fallback_configured() {
        let selector = ProviderSelector::new(Box::new(AlwaysFails("primary")));
        let result = selector.complete(&[], CompletionOptions::default()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn propagates_fallback_error_when_both_fail() {
        let selector = ProviderSelector::new(Box::new(AlwaysFails("primary")))
            .with_fallback(Box::new(AlwaysFails("fallback")));
        let result = selector.complete(&[], CompletionOptions::default()).await;
        assert!(result.is_err());
    }
}
