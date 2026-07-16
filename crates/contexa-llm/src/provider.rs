//! `LlmProvider` trait — `docs/02_System_Architecture.md` §7, reconciled
//! with `docs/08_AI_Orchestrator.md` §7's fuller `complete` signature
//! (messages + options, not just a pre-assembled prompt).

use async_trait::async_trait;

use contexa_core::Result;

use crate::types::{CompletionOptions, Message, ResponseStream};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// # Errors
    /// Returns an error if the provider can't be reached or rejects the request.
    async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream>;
    fn max_tokens(&self) -> usize;
    fn supports_streaming(&self) -> bool;
    /// Used in fallback logging (docs/08 §7) — not a stable identifier.
    fn provider_name(&self) -> &'static str;
}
