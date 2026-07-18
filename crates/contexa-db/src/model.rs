//! DB-layer persistence types — `docs/04_Database_Design.md` §4, §8.1.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ContextChange,
    AppSwitch,
    UserQuery,
    AiResponse,
}

impl EventType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            EventType::ContextChange => "context_change",
            EventType::AppSwitch => "app_switch",
            EventType::UserQuery => "user_query",
            EventType::AiResponse => "ai_response",
        }
    }
}

impl std::str::FromStr for EventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "context_change" => Ok(EventType::ContextChange),
            "app_switch" => Ok(EventType::AppSwitch),
            "user_query" => Ok(EventType::UserQuery),
            "ai_response" => Ok(EventType::AiResponse),
            other => Err(format!("unknown event_type: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryChunk {
    pub id: Uuid,
    pub context_id: Option<Uuid>,
    pub content: String,
    pub content_hash: String,
    pub timestamp: DateTime<Utc>,
    pub application: String,
    pub metadata: HashMap<String, String>,
    pub token_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub summary: String,
    pub application: String,
    pub window_title: String,
    pub duration_ms: Option<i64>,
    pub context_id: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoredChunk {
    pub id: Uuid,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub application: String,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PurgeStats {
    pub chunks_deleted: usize,
    pub snapshots_deleted: usize,
    pub events_deleted: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/// `docs/07_Memory_Engine.md` §9 `MemoryStats` — a DB-layer query result, so
/// it lives here and `contexa-memory` reuses it directly rather than defining
/// (and converting to/from) a duplicate type.
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_chunks: u64,
    pub total_timeline_events: u64,
    pub database_size_mb: f64,
    pub oldest_record: Option<DateTime<Utc>>,
}

/// `docs/11_MCP_Runtime.md` §7.1 `TokenInfo` — a `mcp_tokens` row.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub id: String,
    pub token_hash: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}
