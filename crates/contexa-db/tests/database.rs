//! Integration tests against the real vendored sqlite-vec extension —
//! see plan step 7 / `docs/19_Coding_Standards.md` §4.8.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use contexa_core::{CaptureMethod, ContextSnapshot};
use contexa_db::{
    ContextRepository, Database, EventType, McpRepository, MemoryChunk, MemoryRepository,
    Pagination, SqliteContextRepository, SqliteMcpRepository, SqliteMemoryRepository,
    SqliteTimelineRepository, TimeRange, TimelineEvent, TimelineRepository,
};
use uuid::Uuid;

fn open_test_db() -> (tempfile::TempDir, Arc<Database>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.sqlite3");
    let db = Database::open(&db_path, None).expect("open database");
    (dir, Arc::new(db))
}

fn sample_snapshot() -> ContextSnapshot {
    ContextSnapshot {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        window_title: "main.rs — VS Code".to_string(),
        process_name: "Code.exe".to_string(),
        process_id: 1234,
        hwnd: Some(5678),
        url: None,
        document_path: Some("D:\\Contexa\\crates\\contexa-db\\src\\lib.rs".to_string()),
        visible_text: Some("pub struct Database".to_string()),
        selected_text: None,
        metadata: HashMap::from([("language".to_string(), "rust".to_string())]),
        language: Some("rust".to_string()),
        capture_method: CaptureMethod::Uia,
    }
}

#[tokio::test]
async fn context_repository_round_trips_a_snapshot() {
    let (_dir, db) = open_test_db();
    let repo = SqliteContextRepository(db);
    let snapshot = sample_snapshot();

    repo.insert_snapshot(&snapshot)
        .await
        .expect("insert_snapshot");

    let fetched = repo
        .get_snapshot(&snapshot.id.to_string())
        .await
        .expect("get_snapshot")
        .expect("snapshot should exist");
    assert_eq!(fetched.id, snapshot.id);
    assert_eq!(fetched.window_title, snapshot.window_title);
    assert_eq!(fetched.capture_method, CaptureMethod::Uia);
    assert_eq!(fetched.metadata.get("language"), Some(&"rust".to_string()));

    let recent = repo.get_recent(60).await.expect("get_recent");
    assert!(recent.iter().any(|s| s.id == snapshot.id));
}

#[tokio::test]
async fn memory_repository_searches_and_purges() {
    let (_dir, db) = open_test_db();
    let repo = SqliteMemoryRepository(db);

    let chunk_id = Uuid::new_v4();
    let chunk = MemoryChunk {
        id: chunk_id,
        context_id: None,
        content: "OAuth 2.0 authorization code flow".to_string(),
        content_hash: "hash-1".to_string(),
        timestamp: Utc::now(),
        application: "Code.exe".to_string(),
        metadata: HashMap::new(),
        token_count: 6,
    };
    repo.insert_chunk(&chunk).await.expect("insert_chunk");

    let vector = vec![0.1f32; 384];
    repo.insert_embedding(&chunk_id.to_string(), &vector, "all-MiniLM-L6-v2")
        .await
        .expect("insert_embedding");

    let results = repo
        .search_similar(&vector, 5, 0.01)
        .await
        .expect("search_similar");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, chunk_id);
    assert!(results[0].distance < 0.01);

    let stats = repo
        .purge_before(Utc::now() + Duration::days(1))
        .await
        .expect("purge_before");
    assert_eq!(stats.chunks_deleted, 1);
}

#[tokio::test]
async fn memory_repository_deletes_and_reports_stats() {
    let (_dir, db) = open_test_db();
    let repo = SqliteMemoryRepository(db);

    let make_chunk = |content: &str| MemoryChunk {
        id: Uuid::new_v4(),
        context_id: None,
        content: content.to_string(),
        content_hash: format!("hash-{content}"),
        timestamp: Utc::now(),
        application: "Code.exe".to_string(),
        metadata: HashMap::new(),
        token_count: 4,
    };

    let chunk_a = make_chunk("alpha");
    let chunk_b = make_chunk("beta");
    repo.insert_chunk(&chunk_a).await.expect("insert_chunk a");
    repo.insert_chunk(&chunk_b).await.expect("insert_chunk b");
    let vector = vec![0.2f32; 384];
    repo.insert_embedding(&chunk_a.id.to_string(), &vector, "all-MiniLM-L6-v2")
        .await
        .expect("insert_embedding a");
    repo.insert_embedding(&chunk_b.id.to_string(), &vector, "all-MiniLM-L6-v2")
        .await
        .expect("insert_embedding b");

    let stats = repo.get_stats().await.expect("get_stats");
    assert_eq!(stats.total_chunks, 2);
    assert!(stats.oldest_record.is_some());
    assert!(stats.database_size_mb > 0.0);

    repo.delete_chunk(&chunk_a.id.to_string())
        .await
        .expect("delete_chunk");
    let remaining = repo
        .search_similar(&vector, 10, 1.0)
        .await
        .expect("search_similar after delete_chunk");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, chunk_b.id);

    let deleted = repo.delete_all().await.expect("delete_all");
    assert_eq!(deleted, 1);
    let stats_after = repo.get_stats().await.expect("get_stats after delete_all");
    assert_eq!(stats_after.total_chunks, 0);
}

#[tokio::test]
async fn timeline_repository_returns_a_page_in_range() {
    let (_dir, db) = open_test_db();
    let repo = SqliteTimelineRepository(db);

    let event = TimelineEvent {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        event_type: EventType::AppSwitch,
        summary: "Switched to VS Code".to_string(),
        application: "Code.exe".to_string(),
        window_title: "main.rs — VS Code".to_string(),
        duration_ms: Some(1500),
        context_id: None,
    };
    repo.insert_event(&event).await.expect("insert_event");

    let range = TimeRange {
        start: Utc::now() - Duration::hours(1),
        end: Utc::now() + Duration::hours(1),
    };
    let page = repo
        .get_range(
            range,
            Pagination {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .expect("get_range");

    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, event.id);
}

#[tokio::test]
async fn mcp_repository_manages_tokens_and_audit_log() {
    let (_dir, db) = open_test_db();
    let repo = SqliteMcpRepository(db);

    let id = repo
        .create_token("Cursor", "bcrypt-hash-placeholder")
        .await
        .expect("create_token");

    let active = repo.find_active_tokens().await.expect("find_active_tokens");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, id);
    assert_eq!(active[0].label, "Cursor");
    assert!(!active[0].revoked);
    assert!(active[0].last_used_at.is_none());

    repo.touch_token(&id).await.expect("touch_token");
    let touched = repo.find_active_tokens().await.expect("find_active_tokens");
    assert!(touched[0].last_used_at.is_some());

    repo.log_tool_call(&id, "get_current_context", "{}")
        .await
        .expect("log_tool_call");

    repo.revoke_token(&id).await.expect("revoke_token");
    let after_revoke = repo.find_active_tokens().await.expect("find_active_tokens");
    assert!(after_revoke.is_empty());
}
