//! Anthropic adapter — cloud path in ADR-0007. Requires an API key
//! (`CredentialVault`, key name `anthropic_api_key`); the caller retrieves it
//! and passes it in.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use contexa_core::{ContexaError, Result};

use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream, Role};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";
// Context window, not a queryable value — Claude 3.5 family default (docs/08
// §7 doesn't specify a figure; same "conservative default" rationale as ollama.rs).
const DEFAULT_MAX_TOKENS: usize = 200_000;
// Anthropic's `max_tokens` request field is required (unlike OpenAI's optional
// one) — it caps completion length, not context window.
const DEFAULT_COMPLETION_TOKENS: u32 = 4096;

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
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
struct MessagesRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum StreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: Delta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    text: Option<String>,
}

fn role_str(role: Role) -> &'static str {
    match role {
        // Anthropic has no "system" message role — it's a separate top-level
        // field. System messages are folded into it below, not sent as roles.
        Role::System | Role::User => "user",
        Role::Assistant => "assistant",
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream> {
        let system: Option<String> = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let body = MessagesRequest {
            model: &self.model,
            messages: messages
                .iter()
                .filter(|m| m.role != Role::System)
                .map(|m| ChatMessage {
                    role: role_str(m.role),
                    content: &m.content,
                })
                .collect(),
            system: system.as_deref(),
            max_tokens: opts.max_tokens.unwrap_or(DEFAULT_COMPLETION_TOKENS),
            stream: true,
            temperature: opts.temperature,
        };

        let mut response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
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
                    match serde_json::from_str::<StreamEvent>(payload) {
                        Ok(StreamEvent::ContentBlockDelta { delta }) => {
                            if let Some(text) = delta.text {
                                if !text.is_empty() && tx.send(Ok(text)).is_err() {
                                    return; // receiver dropped
                                }
                            }
                        }
                        Ok(StreamEvent::MessageStop) => return,
                        Ok(StreamEvent::Other) => {}
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
        "anthropic"
    }
}

#[allow(clippy::needless_pass_by_value)]
fn llm_err(e: reqwest::Error) -> ContexaError {
    ContexaError::LlmProviderError {
        provider: "anthropic".to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real shapes from Anthropic's documented streaming Messages API
    // (docs.anthropic.com/en/api/messages-streaming).
    #[test]
    fn parses_a_real_content_block_delta() {
        let payload = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let Ok(parsed) = serde_json::from_str::<StreamEvent>(payload) else {
            panic!("expected the documented shape to parse");
        };
        let StreamEvent::ContentBlockDelta { delta } = parsed else {
            panic!("expected a content_block_delta variant");
        };
        assert_eq!(delta.text.as_deref(), Some("Hello"));
    }

    #[test]
    fn parses_message_stop() {
        let payload = r#"{"type":"message_stop"}"#;
        let Ok(parsed) = serde_json::from_str::<StreamEvent>(payload) else {
            panic!("expected the documented shape to parse");
        };
        assert!(matches!(parsed, StreamEvent::MessageStop));
    }

    #[test]
    fn ignores_unhandled_event_types() {
        let payload = r#"{"type":"message_start","message":{"id":"msg_1"}}"#;
        let Ok(parsed) = serde_json::from_str::<StreamEvent>(payload) else {
            panic!("expected an unrecognized-but-valid event to parse");
        };
        assert!(matches!(parsed, StreamEvent::Other));
    }

    #[test]
    fn system_message_moves_to_top_level_field() {
        let messages = [
            Message {
                role: Role::System,
                content: "be terse".to_string(),
            },
            Message {
                role: Role::User,
                content: "hi".to_string(),
            },
        ];
        let system: Option<String> = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());
        let non_system: Vec<_> = messages.iter().filter(|m| m.role != Role::System).collect();
        assert_eq!(system.as_deref(), Some("be terse"));
        assert_eq!(non_system.len(), 1);
    }

    #[test]
    fn request_serializes_to_documented_shape() {
        let body = MessagesRequest {
            model: "claude-3-5-sonnet-latest",
            messages: vec![ChatMessage {
                role: "user",
                content: "hi",
            }],
            system: Some("be terse"),
            max_tokens: 4096,
            stream: true,
            temperature: Some(0.7),
        };
        let Ok(serialized) = serde_json::to_string(&body) else {
            panic!("expected MessagesRequest to serialize");
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&serialized) else {
            panic!("expected serialized output to be valid json");
        };
        assert_eq!(json["model"], "claude-3-5-sonnet-latest");
        assert_eq!(json["system"], "be terse");
        assert_eq!(json["max_tokens"], 4096);
        assert_eq!(json["stream"], true);
    }
}
