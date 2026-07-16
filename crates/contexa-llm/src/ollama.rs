//! Ollama adapter — ADR-0007's "Recommended" default (local, no API key).
//! The only provider that doesn't need `CredentialVault` at all.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use contexa_core::{ContexaError, Result};

use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream, Role};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
// Conservative default; actual limit is model-dependent (llama3.2:3b ~ 8K
// context per ADR-0007's recommended model) and not queryable via the API.
const DEFAULT_MAX_TOKENS: usize = 8192;

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: model.into(),
        }
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct ChatChunk {
    message: Option<ChatChunkMessage>,
    #[serde(default)]
    done: bool,
}

#[derive(Deserialize)]
struct ChatChunkMessage {
    content: String,
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
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
            options: Some(ChatOptions {
                temperature: opts.temperature,
                num_predict: opts.max_tokens,
            }),
        };

        let mut response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
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
                    Ok(None) => return, // body finished without an explicit "done" line
                    Err(e) => {
                        let _ = tx.send(Err(llm_err(e)));
                        return;
                    }
                };
                buf.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].to_string();
                    buf.drain(..=pos);
                    if line.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<ChatChunk>(&line) {
                        Ok(parsed) => {
                            if let Some(msg) = parsed.message {
                                if !msg.content.is_empty() && tx.send(Ok(msg.content)).is_err() {
                                    return; // receiver dropped — stop reading early
                                }
                            }
                            if parsed.done {
                                return;
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
        "ollama"
    }
}

// reqwest::Error is cheap to take by value (map_err's closure arrives owned
// regardless) — same rationale as contexa-vision's win_err.
#[allow(clippy::needless_pass_by_value)]
fn llm_err(e: reqwest::Error) -> ContexaError {
    ContexaError::LlmProviderError {
        provider: "ollama".to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real shapes from Ollama's documented /api/chat streaming response
    // (github.com/ollama/ollama/blob/main/docs/api.md) — verifies the
    // hand-rolled `ChatChunk`/`ChatChunkMessage` deserialization without
    // needing a live server.
    #[test]
    fn parses_a_real_streaming_chunk() {
        let line = r#"{"model":"llama3.2","created_at":"2023-08-04T08:52:19.385406455-07:00","message":{"role":"assistant","content":"The"},"done":false}"#;
        let Ok(parsed) = serde_json::from_str::<ChatChunk>(line) else {
            panic!("expected the documented shape to parse");
        };
        assert_eq!(parsed.message.map(|m| m.content), Some("The".to_string()));
        assert!(!parsed.done);
    }

    #[test]
    fn parses_the_final_done_chunk() {
        let line = r#"{"model":"llama3.2","created_at":"2023-08-04T19:22:45.499127Z","message":{"role":"assistant","content":""},"done":true,"total_duration":4883583458,"eval_count":282}"#;
        let Ok(parsed) = serde_json::from_str::<ChatChunk>(line) else {
            panic!("expected the documented shape to parse");
        };
        assert!(parsed.done);
    }

    #[test]
    fn request_serializes_to_documented_shape() {
        let body = ChatRequest {
            model: "llama3.2:3b",
            messages: vec![ChatMessage {
                role: "user",
                content: "hi",
            }],
            stream: true,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(100),
            }),
        };
        let Ok(serialized) = serde_json::to_string(&body) else {
            panic!("expected ChatRequest to serialize");
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&serialized) else {
            panic!("expected serialized output to be valid json");
        };
        assert_eq!(json["model"], "llama3.2:3b");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "hi");
        assert_eq!(json["stream"], true);
        assert_eq!(json["options"]["num_predict"], 100);
    }

    #[test]
    fn options_field_omitted_when_none() {
        let body = ChatRequest {
            model: "llama3.2:3b",
            messages: vec![],
            stream: true,
            options: None,
        };
        let Ok(json) = serde_json::to_string(&body) else {
            panic!("expected ChatRequest to serialize");
        };
        assert!(!json.contains("options"));
    }
}
