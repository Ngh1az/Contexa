//! `OpenAI` adapter — cloud path in ADR-0007. Requires an API key
//! (`CredentialVault`, key name `openai_api_key`); the caller retrieves it
//! and passes it in.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use contexa_core::{ContexaError, Result};

use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream, Role};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
// Context window, not a queryable value — gpt-4o family default (docs/08 §7
// doesn't specify a figure; same "conservative default" rationale as ollama.rs).
const DEFAULT_MAX_TOKENS: usize = 128_000;

pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// # Panics
    /// Panics if `base_url` is plaintext HTTP pointed at a non-loopback host
    /// (see `provider::assert_secure_base_url`).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        crate::provider::assert_secure_base_url(&base_url);
        self.base_url = base_url;
        self
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatChunk {
    choices: Vec<ChatChunkChoice>,
}

#[derive(Deserialize)]
struct ChatChunkChoice {
    delta: ChatChunkDelta,
}

#[derive(Deserialize, Default)]
struct ChatChunkDelta {
    content: Option<String>,
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream> {
        let body = ChatRequest {
            model: &self.model,
            messages: messages
                .iter()
                .map(|m| ChatMessage {
                    role: role_str(m.role),
                    content: &m.content,
                })
                .collect(),
            stream: true,
            max_tokens: opts.max_tokens,
            temperature: opts.temperature,
        };

        let mut response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(llm_err)?;

        if !response.status().is_success() {
            return Err(ContexaError::LlmProviderError {
                provider: self.provider_name().to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut buf = String::new();
            loop {
                let chunk = match response.chunk().await {
                    Ok(Some(bytes)) => bytes,
                    Ok(None) => return,
                    Err(e) => {
                        let _ = tx.send(Err(llm_err(e)));
                        return;
                    }
                };
                buf.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf.drain(..=pos);
                    if line.is_empty() || !line.starts_with("data:") {
                        continue;
                    }
                    let payload = line["data:".len()..].trim();
                    if payload == "[DONE]" {
                        return;
                    }
                    match serde_json::from_str::<ChatChunk>(payload) {
                        Ok(parsed) => {
                            for choice in parsed.choices {
                                if let Some(content) = choice.delta.content {
                                    if !content.is_empty() && tx.send(Ok(content)).is_err() {
                                        return; // receiver dropped
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(ContexaError::Conversion(e.to_string())));
                            return;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn max_tokens(&self) -> usize {
        DEFAULT_MAX_TOKENS
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }
}

#[allow(clippy::needless_pass_by_value)]
fn llm_err(e: reqwest::Error) -> ContexaError {
    ContexaError::LlmProviderError {
        provider: "openai".to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real shape from OpenAI's documented streaming chat completion chunk
    // (platform.openai.com/docs/api-reference/chat/streaming).
    #[test]
    fn parses_a_real_streaming_chunk() {
        let payload = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let Ok(parsed) = serde_json::from_str::<ChatChunk>(payload) else {
            panic!("expected the documented shape to parse");
        };
        assert_eq!(parsed.choices[0].delta.content.as_deref(), Some("Hello"));
    }

    #[test]
    fn parses_the_final_empty_delta_chunk() {
        let payload = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        let Ok(parsed) = serde_json::from_str::<ChatChunk>(payload) else {
            panic!("expected the documented shape to parse");
        };
        assert!(parsed.choices[0].delta.content.is_none());
    }

    #[test]
    fn request_serializes_to_documented_shape() {
        let body = ChatRequest {
            model: "gpt-4o",
            messages: vec![ChatMessage {
                role: "user",
                content: "hi",
            }],
            stream: true,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let Ok(serialized) = serde_json::to_string(&body) else {
            panic!("expected ChatRequest to serialize");
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&serialized) else {
            panic!("expected serialized output to be valid json");
        };
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["stream"], true);
        assert_eq!(json["max_tokens"], 100);
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        let body = ChatRequest {
            model: "gpt-4o",
            messages: vec![],
            stream: true,
            max_tokens: None,
            temperature: None,
        };
        let Ok(json) = serde_json::to_string(&body) else {
            panic!("expected ChatRequest to serialize");
        };
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("temperature"));
    }
}
