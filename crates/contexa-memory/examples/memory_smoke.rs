//! Manual smoke test — NOT part of `cargo test` (needs the real fastembed
//! ONNX model, which downloads on first run). Run:
//!
//! ```powershell
//! cargo run -p contexa-memory --example memory_smoke
//! ```
//!
//! Unlike `contexa-vision`/`contexa-context`'s smoke examples, this one
//! needs no live window — it's fully self-contained (temp `SQLite` DB +
//! synthetic `ContextSnapshot`) and asserts its own results.
#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use uuid::Uuid;

use contexa_core::{CaptureMethod, ContextSnapshot};
use contexa_db::{
    Database, SqliteContextRepository, SqliteMemoryRepository, SqliteTimelineRepository, TimeRange,
};
use contexa_memory::{ContexaMemoryEngine, MemoryEngine, SearchOptions};

#[tokio::main]
async fn main() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("memory_smoke.sqlite3");
    let db = Arc::new(Database::open(&db_path, None).expect("open database"));

    let context_repo = Arc::new(SqliteContextRepository(Arc::clone(&db)));
    let memory_repo = Arc::new(SqliteMemoryRepository(Arc::clone(&db)));
    let timeline_repo = Arc::new(SqliteTimelineRepository(Arc::clone(&db)));

    println!("loading fastembed model (downloads on first run)...");
    let engine = ContexaMemoryEngine::new(context_repo, memory_repo, timeline_repo)
        .expect("build memory engine");
    println!("model loaded.");

    let snapshot = ContextSnapshot {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        window_title: "main.rs — VS Code".to_string(),
        process_name: "Code.exe".to_string(),
        process_id: 1234,
        hwnd: Some(5678),
        url: None,
        document_path: Some(
            "D:\\Contexa\\crates\\contexa-memory\\src\\engine.rs".to_string(),
        ),
        visible_text: Some(
            "The OAuth 2.0 authorization code flow exchanges a code for an access token \
             via the token endpoint, using the client secret for authentication."
                .to_string(),
        ),
        selected_text: None,
        metadata: HashMap::new(),
        language: Some("rust".to_string()),
        capture_method: CaptureMethod::Uia,
    };

    engine.ingest(&snapshot).await.expect("ingest");

    // The embedding pipeline batches (flush at 10 chunks, or every 5s via the
    // periodic timer) — `ContexaMemoryEngine` doesn't expose `flush()`
    // directly (an internal wiring detail, not part of the `MemoryEngine`
    // trait), so poll search briefly instead of asserting immediately.
    //
    // `min_score: 0.5` here, not `SearchOptions::default()`'s 0.7 — measured
    // against the real fastembed model, this paraphrased query lands at
    // cosine distance ~0.42 (similarity ~0.58) from the matching chunk.
    // `docs/07` §9's documented default of 0.7 would filter out this exact,
    // genuinely-relevant match. Flagging as a real calibration gap in the
    // spec's default rather than silently changing it.
    let opts = SearchOptions {
        min_score: 0.5,
        ..SearchOptions::default()
    };
    let mut hits = Vec::new();
    for _ in 0..20 {
        hits = engine
            .search("how does OAuth token exchange work", opts.clone())
            .await
            .expect("search");
        if !hits.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    assert!(
        !hits.is_empty(),
        "expected semantic search to find the ingested chunk within 10s"
    );
    println!("found {} match(es); top: {:?}", hits.len(), hits[0].content);
    assert!(hits[0].content.contains("OAuth"));

    let stats = engine.get_stats().await.expect("get_stats");
    println!("stats: {stats:?}");
    assert_eq!(stats.total_chunks, 1);

    let timeline = engine
        .get_timeline(TimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now() + chrono::Duration::hours(1),
        })
        .await
        .expect("get_timeline");
    assert_eq!(timeline.len(), 1);

    println!("PASS");
}
