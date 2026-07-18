//! Gemini adapter — cloud path in ADR-0007. Requires an API key
//! (`CredentialVault`, key name `gemini_api_key`); the caller retrieves it
//! and passes it in.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use contexa_core::{ContexaError, Result};

use crate::provider::LlmProvider;
use crate::types::{CompletionOptions, Message, ResponseStream, Role};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
// Context window, not a queryable value — Gemini 1.5 Flash/Pro family default
// (docs/08 §7 doesn't specify a figure; same "conservative default" rationale
// as ollama.rs).
const DEFAULT_MAX_TOKENS: usize = 1_000_000;

pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl GeminiProvider {
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
struct GenerateRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<SystemInstruction<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content<'a> {
    role: &'a str,
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct SystemInstruction<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct GenerateChunk {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ChunkPart>,
}

#[derive(Deserialize)]
struct ChunkPart {
    #[serde(default)]
    text: Option<String>,
}

fn role_str(role: Role) -> &'static str {
    match role {
        // Gemini's `contents` roles are "user"/"model" only — system prompts
        // go in the separate `systemInstruction` field, folded in below.
        Role::System | Role::User => "user",
        Role::Assistant => "model",
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, messages: &[Message], opts: CompletionOptions) -> Result<ResponseStream> {
        let system_text = messages.iter().find(|m| m.role == Role::System).map(|m| m.content.as_str());

        let body = GenerateRequest {
            contents: messages
                .iter()
                .filter(|m| m.role != Role::System)
                .map(|m| Content {
                    role: role_str(m.role),
                    parts: vec![Part { text: &m.content }],
                })
                .collect(),
            system_instruction: system_text.map(|text| SystemInstruction {
                parts: vec![Part { text }],
            }),
            generation_config: GenerationConfig {
                max_output_tokens: opts.max_tokens,
                temperature: opts.temperature,
            },
        };

        let mut response = self
            .client
            .post(format!(
                "{}/models/{}:streamGenerateContent",
                self.base_url, self.model
            ))
            // Key goes in a header, not `?key=` — a query-string key ends up
            // in request URLs that proxies/reqwest error messages/tracing
            // spans may log; `x-goog-api-key` is Gemini's documented
            // alternative (ai.google.dev/gemini-api/docs/api-key).
            .query(&[("alt", "sse")])
            .header("x-goog-api-key", &self.api_key)
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
                    match serde_json::from_str::<GenerateChunk>(payload) {
                        Ok(parsed) => {
                            for candidate in parsed.candidates {
                                let Some(content) = candidate.content else { continue };
                                for part in content.parts {
                                    if let Some(text) = part.text {
                                        if !text.is_empty() && tx.send(Ok(text)).is_err() {
                                            return; // receiver dropped
                                        }
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
        "gemini"
    }
}

#[allow(clippy::needless_pass_by_value)]
fn llm_err(e: reqwest::Error) -> ContexaError {
    ContexaError::LlmProviderError {
        provider: "gemini".to_string(),
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real shape from Gemini's documented `streamGenerateContent` SSE chunk
    // (ai.google.dev/api/generate-content#generatecontentresponse).
    #[test]
    fn parses_a_real_streaming_chunk() {
        let payload = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"},"index":0}]}"#;
        let Ok(parsed) = serde_json::from_str::<GenerateChunk>(payload) else {
            panic!("expected the documented shape to parse");
        };
        assert_eq!(
            parsed.candidates[0]
                .content
                .as_ref()
                .and_then(|c| c.parts[0].text.as_deref()),
            Some("Hello")
        );
    }

    #[test]
    fn request_serializes_to_documented_shape() {
        let body = GenerateRequest {
            contents: vec![Content {
                role: "user",
                parts: vec![Part { text: "hi" }],
            }],
            system_instruction: Some(SystemInstruction {
                parts: vec![Part { text: "be terse" }],
            }),
            generation_config: GenerationConfig {
                max_output_tokens: Some(100),
                temperature: Some(0.7),
            },
        };
        let Ok(serialized) = serde_json::to_string(&body) else {
            panic!("expected GenerateRequest to serialize");
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&serialized) else {
            panic!("expected serialized output to be valid json");
        };
        assert_eq!(json["contents"][0]["role"], "user");
        assert_eq!(json["contents"][0]["parts"][0]["text"], "hi");
        assert_eq!(json["systemInstruction"]["parts"][0]["text"], "be terse");
        assert_eq!(json["generationConfig"]["maxOutputTokens"], 100);
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        let body = GenerateRequest {
            contents: vec![],
            system_instruction: None,
            generation_config: GenerationConfig {
                max_output_tokens: None,
                temperature: None,
            },
        };
        let Ok(json) = serde_json::to_string(&body) else {
            panic!("expected GenerateRequest to serialize");
        };
        assert!(!json.contains("systemInstruction"));
        assert!(!json.contains("maxOutputTokens"));
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn assistant_role_maps_to_model() {
        assert_eq!(role_str(Role::Assistant), "model");
        assert_eq!(role_str(Role::User), "user");
    }
}
