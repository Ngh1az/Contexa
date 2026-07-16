//! Manual smoke test — NOT part of `cargo test`. Needs a real running
//! Ollama instance (`ollama serve`, default port 11434), which an automated
//! test run can't guarantee — same reasoning as `contexa-vision`'s
//! `vision_smoke.rs` needing a live window.
//!
//! ```powershell
//! cargo run -p contexa-llm --example llm_smoke
//! ```

use contexa_llm::{CompletionOptions, LlmProvider, Message, OllamaProvider, Role};

#[tokio::main]
async fn main() {
    let provider = OllamaProvider::new("llama3.2:3b"); // ADR-0007 recommended default

    let messages = vec![Message {
        role: Role::User,
        content: "Reply with exactly one short sentence confirming you received this.".to_string(),
    }];

    let started = std::time::Instant::now();
    let mut rx = match provider.complete(&messages, CompletionOptions::default()).await {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!(
                "complete() failed: {e}\n(expected if Ollama isn't running — see docs/22 for install steps, or `ollama serve` + `ollama pull llama3.2:3b`)"
            );
            return;
        }
    };

    let mut first_token = None;
    let mut full_response = String::new();
    while let Some(chunk) = rx.recv().await {
        match chunk {
            Ok(text) => {
                if first_token.is_none() {
                    first_token = Some(started.elapsed());
                }
                full_response.push_str(&text);
            }
            Err(e) => {
                eprintln!("stream error: {e}");
                return;
            }
        }
    }

    match first_token {
        Some(ttft) => println!("time to first token: {ttft:?} (target < 1s, docs/08 §11)"),
        None => println!("no tokens received (empty response?)"),
    }
    println!("full response: {full_response:?}");
}
