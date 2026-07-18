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

/// Guards `with_base_url` on API-key-bearing providers (openai/anthropic/gemini)
/// against being pointed at plaintext HTTP — the key would then travel the
/// wire unencrypted. `http://localhost`/`127.0.0.1`/`[::1]` stay allowed so
/// tests can point at a local mock server.
///
/// Panics rather than returning a `Result`: this is a caller configuration
/// mistake, not a runtime condition — same rationale as other builder-time
/// invariant checks (e.g. `assert!` in `Vec::with_capacity` overflow paths).
pub(crate) fn assert_secure_base_url(url: &str) {
    let is_loopback = url.starts_with("http://localhost")
        || url.starts_with("http://127.0.0.1")
        || url.starts_with("http://[::1]");
    assert!(
        !url.starts_with("http://") || is_loopback,
        "insecure base_url {url:?} for a cloud LLM provider that sends an API \
         key — use https://, or http://localhost / http://127.0.0.1 for a local test mock"
    );
}

#[cfg(test)]
mod tests {
    use super::assert_secure_base_url;

    #[test]
    fn https_is_allowed() {
        assert_secure_base_url("https://api.openai.com/v1");
    }

    #[test]
    fn loopback_http_is_allowed() {
        assert_secure_base_url("http://localhost:1234/v1");
        assert_secure_base_url("http://127.0.0.1:1234/v1");
    }

    #[test]
    #[should_panic(expected = "insecure base_url")]
    fn non_loopback_http_panics() {
        assert_secure_base_url("http://api.openai.com/v1");
    }
}
