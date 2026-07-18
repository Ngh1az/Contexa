//! Manual smoke test — NOT part of `cargo test` (needs a live foreground
//! window for real capture, same reasoning as `contexa-vision`'s
//! `vision_smoke.rs`, plus optionally a running Ollama for a real LLM
//! response). Exercises the same wiring `apps/desktop/src-tauri/src/lib.rs`
//! does, minus Tauri itself: real window capture -> Context Engine -> real
//! temp DB -> Memory Engine -> Orchestrator -> `handle_request` ->
//! `take_stream`. Prints either streamed tokens (Ollama running) or a clean
//! error (Ollama not running) — either way, confirms the pipeline runs end
//! to end up to the LLM boundary without panicking.
//!
//! ```powershell
//! cargo run -p contexa-orchestrator --example orchestrator_smoke
//! ```
#![allow(clippy::expect_used)]

use std::sync::Arc;

use uuid::Uuid;

use contexa_context::{ContexaContextEngine, ContextEngine};
use contexa_core::{RequestAction, RequestPreferences, UserRequest};
use contexa_db::{Database, SqliteContextRepository, SqliteMemoryRepository, SqliteTimelineRepository};
use contexa_llm::{OllamaProvider, ProviderSelector};
use contexa_memory::{ContexaMemoryEngine, MemoryEngine};
use contexa_orchestrator::{AiOrchestrator, ContexaOrchestrator, PipelineConfig, PipelineManager};
use contexa_prompt::ContexaPromptBuilder;
use contexa_search::ContexaSearchEngine;
use contexa_vision::{ContexaVisionEngine, VisionEngine};

#[tokio::main]
async fn main() {
    println!("capturing the real active window...");
    let (vision, _rx) = ContexaVisionEngine::new(Vec::new());
    let vision_result = vision
        .capture_active_window()
        .expect("capture_active_window (needs a live foreground window)");
    println!(
        "captured: process={} title={:?}",
        vision_result.process_name, vision_result.window_title
    );

    let context = Arc::new(ContexaContextEngine::with_builtin_enrichers());
    let snapshot = context
        .process_vision_result(vision_result)
        .expect("process_vision_result")
        .expect("first capture should always produce a snapshot");
    println!("snapshot ready: id={}", snapshot.id);

    let dir = tempfile::tempdir().expect("tempdir");
    let db = Arc::new(Database::open(&dir.path().join("smoke.sqlite3"), None).expect("open database"));
    let context_repo = Arc::new(SqliteContextRepository(Arc::clone(&db)));
    let memory_repo = Arc::new(SqliteMemoryRepository(Arc::clone(&db)));
    let timeline_repo = Arc::new(SqliteTimelineRepository(Arc::clone(&db)));

    println!("loading fastembed model (downloads on first run)...");
    let memory_engine: Arc<dyn MemoryEngine> = Arc::new(
        ContexaMemoryEngine::new(context_repo, memory_repo, timeline_repo).expect("build memory engine"),
    );
    memory_engine.ingest(&snapshot).await.expect("ingest");
    println!("memory engine ready.");

    let prompt_builder = Arc::new(ContexaPromptBuilder::default());
    let provider = ProviderSelector::new(Box::new(OllamaProvider::new("qwen3:8b")));
    let search_engine = Arc::new(ContexaSearchEngine::default()); // disabled, not needed for this run
    let vision: Arc<dyn VisionEngine> = Arc::new(vision);

    let pipeline = Arc::new(PipelineManager::new(
        Arc::clone(&context) as Arc<dyn ContextEngine>,
        memory_engine,
        vision,
        search_engine,
        prompt_builder,
        provider,
        PipelineConfig::default(),
    ));
    let orchestrator = ContexaOrchestrator::new(context as Arc<dyn ContextEngine>, pipeline);

    let request = UserRequest {
        id: Uuid::new_v4(),
        action: RequestAction::Explain,
        query: None,
        context_override: None,
        preferences: RequestPreferences::default(),
    };

    println!("calling handle_request (Explain)...");
    let handle = orchestrator.handle_request(request).await.expect("handle_request");
    println!("request accepted: id={}", handle.id);

    let Some(mut stream) = orchestrator.take_stream(&handle.id).await else {
        let status = orchestrator
            .get_active_requests()
            .into_iter()
            .find(|h| h.id == handle.id)
            .map(|h| h.status);
        println!("no stream produced; final status = {status:?}");
        println!("PASS if that's a clean provider-connection failure (e.g. Ollama not running)");
        return;
    };

    println!("--- streamed response ---");
    let mut got_any_chunk = false;
    loop {
        match stream.recv().await {
            Some(Ok(chunk)) => {
                got_any_chunk = true;
                print!("{chunk}");
            }
            Some(Err(e)) => {
                println!("\n--- stream ended with a clean error (expected if Ollama isn't running): {e} ---");
                println!("PASS (pipeline ran end-to-end to the LLM boundary; provider call failed cleanly)");
                return;
            }
            None => break,
        }
    }
    println!("\n--- end of stream ---");
    assert!(got_any_chunk, "expected at least one token chunk from a real Ollama response");
    println!("PASS (full end-to-end response received from a real Ollama call)");
}
