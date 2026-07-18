//! Manual smoke test — NOT part of `cargo test` (hits the real network,
//! same convention as the other `*_smoke.rs` examples). Confirms the
//! `DuckDuckGoAdapter`'s HTML-scraping assumptions still hold against the
//! live endpoint (ADR-0011 flags markup breakage as a real risk).
//!
//! ```powershell
//! cargo run -p contexa-search --example search_smoke
//! ```
#![allow(clippy::expect_used)]

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use contexa_core::{CaptureMethod, ContextSnapshot};
use contexa_search::{ContexaSearchEngine, SearchEngine};

#[tokio::main]
async fn main() {
    let engine = ContexaSearchEngine::new(true); // opt in explicitly for this manual run

    let context = ContextSnapshot {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        window_title: "smoke test".to_string(),
        process_name: "search_smoke.exe".to_string(),
        process_id: 1,
        hwnd: None,
        url: None,
        document_path: None,
        visible_text: None,
        selected_text: None,
        metadata: HashMap::new(),
        language: None,
        capture_method: CaptureMethod::Uia,
    };

    let response = engine
        .search("Rust tokio spawn_blocking", &context)
        .await
        .expect("search should succeed against the live DuckDuckGo endpoint");

    println!(
        "provider={} query_used={:?} cached={} latency_ms={}",
        response.provider, response.query_used, response.cached, response.latency_ms
    );
    println!("{} result(s):", response.results.len());
    for result in &response.results {
        println!("- {} ({})", result.title, result.url);
        println!("  {}", result.snippet);
    }

    assert!(
        !response.results.is_empty(),
        "expected at least one real result. If this fails: as of 2026-07-18, \
         html.duckduckgo.com/lite.duckduckgo.com return HTTP 202 with an \
         anomaly.js bot-detection challenge page for plain HTTP clients (no \
         markup to parse at all, even with a real browser User-Agent) — this \
         is not a selector bug in duckduckgo.rs's parse_results, it's \
         DuckDuckGo actively blocking the request. See the session notes for \
         details; ADR-0011's zero-config scraping assumption may need revisiting."
    );

    // Second call with the same query should hit the cache.
    let cached = engine
        .search("Rust tokio spawn_blocking", &context)
        .await
        .expect("second search should succeed");
    assert!(cached.cached, "identical query should hit the cache on the second call");

    println!("PASS");
}
