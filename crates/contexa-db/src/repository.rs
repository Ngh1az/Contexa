//! Database repository traits + rusqlite-backed implementations —
//! `docs/04_Database_Design.md` §7 (query patterns), §8.1 (trait signatures).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use contexa_core::{CaptureMethod, ContexaError, ContextSnapshot, Result};

use crate::database::Database;
use crate::model::{
    EventType, MemoryChunk, MemoryStats, Page, Pagination, PurgeStats, ScoredChunk, TimeRange,
    TimelineEvent, TokenInfo,
};

#[async_trait]
pub trait ContextRepository: Send + Sync {
    async fn insert_snapshot(&self, snapshot: &ContextSnapshot) -> Result<()>;
    async fn get_snapshot(&self, id: &str) -> Result<Option<ContextSnapshot>>;
    async fn get_recent(&self, minutes: u32) -> Result<Vec<ContextSnapshot>>;
}

#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn insert_chunk(&self, chunk: &MemoryChunk) -> Result<()>;
    async fn insert_embedding(&self, chunk_id: &str, vector: &[f32], model: &str) -> Result<()>;
    async fn search_similar(
        &self,
        vector: &[f32],
        limit: usize,
        max_distance: f32,
    ) -> Result<Vec<ScoredChunk>>;
    async fn purge_before(&self, date: DateTime<Utc>) -> Result<PurgeStats>;
    /// # Errors
    /// Returns an error if the delete fails.
    async fn delete_chunk(&self, id: &str) -> Result<()>;
    /// # Errors
    /// Returns an error if the delete fails.
    async fn delete_all(&self) -> Result<u64>;
    /// # Errors
    /// Returns an error if the stats queries fail.
    async fn get_stats(&self) -> Result<MemoryStats>;
}

#[async_trait]
pub trait McpRepository: Send + Sync {
    /// Returns the generated token id.
    /// # Errors
    /// Returns an error if the insert fails.
    async fn create_token(&self, label: &str, token_hash: &str) -> Result<String>;
    /// # Errors
    /// Returns an error if the query fails.
    async fn find_active_tokens(&self) -> Result<Vec<TokenInfo>>;
    /// # Errors
    /// Returns an error if the update fails.
    async fn touch_token(&self, id: &str) -> Result<()>;
    /// # Errors
    /// Returns an error if the update fails.
    async fn revoke_token(&self, id: &str) -> Result<()>;
    /// # Errors
    /// Returns an error if the insert fails.
    async fn log_tool_call(&self, token_id: &str, tool_name: &str, request_summary: &str) -> Result<()>;
}

#[async_trait]
pub trait TimelineRepository: Send + Sync {
    async fn insert_event(&self, event: &TimelineEvent) -> Result<()>;
    async fn get_range(
        &self,
        range: TimeRange,
        pagination: Pagination,
    ) -> Result<Page<TimelineEvent>>;
}

/// Runs a blocking DB closure on the Tokio blocking pool, per ADR-0010's async wrapper pattern.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ContexaError::TaskJoin(e.to_string()))?
}

fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ContexaError::Conversion(e.to_string()))
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| ContexaError::Conversion(e.to_string()))
}

fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

// ---------------------------------------------------------------------------
// ContextRepository
// ---------------------------------------------------------------------------

pub struct SqliteContextRepository(pub Arc<Database>);

/// Raw `context_snapshots` row, column order per `CONTEXT_SNAPSHOT_COLUMNS`.
type ContextSnapshotRow = (
    String,
    String,
    String,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    String,
);

fn row_to_context_snapshot(row: ContextSnapshotRow) -> Result<ContextSnapshot> {
    let (
        id,
        timestamp,
        window_title,
        process_name,
        process_id,
        hwnd,
        url,
        document_path,
        visible_text,
        selected_text,
        metadata_json,
        language,
        capture_method,
    ) = row;
    let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json)?;
    Ok(ContextSnapshot {
        id: parse_uuid(&id)?,
        timestamp: parse_timestamp(&timestamp)?,
        window_title,
        process_name,
        process_id,
        hwnd,
        url,
        document_path,
        visible_text,
        selected_text,
        metadata,
        language,
        capture_method: capture_method
            .parse::<CaptureMethod>()
            .map_err(ContexaError::Conversion)?,
    })
}

fn query_context_snapshot_row(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> rusqlite::Result<Option<ContextSnapshotRow>> {
    conn.query_row(sql, params, |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
            row.get(11)?,
            row.get(12)?,
        ))
    })
    .optional()
}

const CONTEXT_SNAPSHOT_COLUMNS: &str = "id, timestamp, window_title, process_name, process_id, \
    hwnd, url, document_path, visible_text, selected_text, metadata_json, language, capture_method";

#[async_trait]
impl ContextRepository for SqliteContextRepository {
    async fn insert_snapshot(&self, snapshot: &ContextSnapshot) -> Result<()> {
        let db = self.0.clone();
        let snapshot = snapshot.clone();
        blocking(move || {
            let metadata_json = serde_json::to_string(&snapshot.metadata)?;
            db.with_write(|conn| {
                conn.execute(
                    "INSERT INTO context_snapshots
                        (id, timestamp, window_title, process_name, process_id, hwnd, url,
                         document_path, visible_text, selected_text, metadata_json, language,
                         capture_method)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        snapshot.id.to_string(),
                        snapshot.timestamp.to_rfc3339(),
                        snapshot.window_title,
                        snapshot.process_name,
                        snapshot.process_id,
                        snapshot.hwnd,
                        snapshot.url,
                        snapshot.document_path,
                        snapshot.visible_text,
                        snapshot.selected_text,
                        metadata_json,
                        snapshot.language,
                        snapshot.capture_method.as_str(),
                    ],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn get_snapshot(&self, id: &str) -> Result<Option<ContextSnapshot>> {
        let db = self.0.clone();
        let id = id.to_string();
        blocking(move || {
            let row = db.with_read(|conn| {
                Ok(query_context_snapshot_row(
                    conn,
                    &format!(
                        "SELECT {CONTEXT_SNAPSHOT_COLUMNS} FROM context_snapshots WHERE id = ?1"
                    ),
                    &[&id],
                )?)
            })?;
            row.map(row_to_context_snapshot).transpose()
        })
        .await
    }

    async fn get_recent(&self, minutes: u32) -> Result<Vec<ContextSnapshot>> {
        let db = self.0.clone();
        let cutoff = (Utc::now() - chrono::Duration::minutes(i64::from(minutes))).to_rfc3339();
        blocking(move || {
            let rows = db.with_read(|conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT {CONTEXT_SNAPSHOT_COLUMNS} FROM context_snapshots \
                     WHERE timestamp > ?1 ORDER BY timestamp DESC"
                ))?;
                let rows = stmt
                    .query_map(params![cutoff], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, Option<i64>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                            row.get::<_, Option<String>>(9)?,
                            row.get::<_, String>(10)?,
                            row.get::<_, Option<String>>(11)?,
                            row.get::<_, String>(12)?,
                        ))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })?;
            rows.into_iter().map(row_to_context_snapshot).collect()
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// MemoryRepository
// ---------------------------------------------------------------------------

pub struct SqliteMemoryRepository(pub Arc<Database>);

type ChunkRow = (Uuid, String, DateTime<Utc>, String);

fn fetch_chunk(conn: &Connection, chunk_id: &str) -> Result<Option<ChunkRow>> {
    let row = conn
        .query_row(
            "SELECT id, content, timestamp, application FROM memory_chunks WHERE id = ?1",
            params![chunk_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;

    row.map(|(id, content, timestamp, application)| {
        Ok((
            parse_uuid(&id)?,
            content,
            parse_timestamp(&timestamp)?,
            application,
        ))
    })
    .transpose()
}

#[async_trait]
impl MemoryRepository for SqliteMemoryRepository {
    async fn insert_chunk(&self, chunk: &MemoryChunk) -> Result<()> {
        let db = self.0.clone();
        let chunk = chunk.clone();
        blocking(move || {
            let metadata_json = serde_json::to_string(&chunk.metadata)?;
            db.with_write(|conn| {
                conn.execute(
                    "INSERT INTO memory_chunks
                        (id, context_id, content, content_hash, timestamp, application,
                         metadata_json, token_count)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        chunk.id.to_string(),
                        chunk.context_id.map(|id| id.to_string()),
                        chunk.content,
                        chunk.content_hash,
                        chunk.timestamp.to_rfc3339(),
                        chunk.application,
                        metadata_json,
                        chunk.token_count,
                    ],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn insert_embedding(&self, chunk_id: &str, vector: &[f32], model: &str) -> Result<()> {
        let db = self.0.clone();
        let chunk_id = chunk_id.to_string();
        let model = model.to_string();
        let blob = vec_to_blob(vector);
        let (table, dimensions): (&str, i64) = match vector.len() {
            384 => ("embeddings", 384),
            768 => ("embeddings_768", 768),
            other => {
                return Err(ContexaError::Conversion(format!(
                    "unsupported embedding dimensions: {other} (expected 384 or 768)"
                )))
            }
        };
        blocking(move || {
            db.with_write(|conn| {
                conn.execute(
                    &format!("INSERT INTO {table} (chunk_id, embedding) VALUES (?1, ?2)"),
                    params![chunk_id, blob],
                )?;
                conn.execute(
                    "INSERT INTO embedding_meta (chunk_id, model, dimensions) VALUES (?1, ?2, ?3)",
                    params![chunk_id, model, dimensions],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn search_similar(
        &self,
        vector: &[f32],
        limit: usize,
        max_distance: f32,
    ) -> Result<Vec<ScoredChunk>> {
        let db = self.0.clone();
        let blob = vec_to_blob(vector);
        // usize -> i64: SQLite LIMIT is signed and a negative value means "no limit",
        // so clamp rather than let an out-of-range cast wrap into a negative number.
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let table = match vector.len() {
            384 => "embeddings",
            768 => "embeddings_768",
            other => {
                return Err(ContexaError::Conversion(format!(
                    "unsupported query vector dimensions: {other} (expected 384 or 768)"
                )))
            }
        };
        blocking(move || {
            db.with_read(|conn| {
                // KNN form validated in spikes/SP-04-sqlite-vec (MATCH + ORDER BY distance
                // avoids a full scan); max_distance is applied after the KNN cut, and the
                // chunk row is fetched separately rather than joined in the same statement.
                let mut stmt = conn.prepare(&format!(
                    "SELECT chunk_id, distance FROM {table} WHERE embedding MATCH ?1 \
                     ORDER BY distance LIMIT ?2"
                ))?;
                let hits = stmt
                    .query_map(params![blob, limit_i64], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;

                let mut scored = Vec::new();
                for (chunk_id, distance) in hits {
                    if distance >= max_distance {
                        continue;
                    }
                    if let Some((id, content, timestamp, application)) =
                        fetch_chunk(conn, &chunk_id)?
                    {
                        scored.push(ScoredChunk {
                            id,
                            content,
                            timestamp,
                            application,
                            distance,
                        });
                    }
                }
                Ok(scored)
            })
        })
        .await
    }

    async fn purge_before(&self, date: DateTime<Utc>) -> Result<PurgeStats> {
        let db = self.0.clone();
        let cutoff = date.to_rfc3339();
        blocking(move || {
            db.with_write(|conn| {
                let tx = conn.unchecked_transaction()?;
                let chunks_deleted =
                    tx.execute("DELETE FROM memory_chunks WHERE timestamp < ?1", params![cutoff])?;
                let snapshots_deleted = tx.execute(
                    "DELETE FROM context_snapshots \
                     WHERE timestamp < ?1 \
                       AND id NOT IN (SELECT context_id FROM memory_chunks WHERE context_id IS NOT NULL)",
                    params![cutoff],
                )?;
                let events_deleted = tx.execute(
                    "DELETE FROM timeline_events WHERE timestamp < ?1",
                    params![cutoff],
                )?;
                tx.commit()?;
                conn.execute_batch("VACUUM;")?;
                Ok(PurgeStats { chunks_deleted, snapshots_deleted, events_deleted })
            })
        })
        .await
    }

    async fn delete_chunk(&self, id: &str) -> Result<()> {
        let db = self.0.clone();
        let id = id.to_string();
        blocking(move || {
            db.with_write(|conn| {
                let tx = conn.unchecked_transaction()?;
                // vec0 virtual tables (embeddings/embeddings_768) have no FK to
                // memory_chunks, so they don't cascade — delete explicitly.
                // embedding_meta does cascade via its own FK.
                tx.execute("DELETE FROM embeddings WHERE chunk_id = ?1", params![id])?;
                tx.execute("DELETE FROM embeddings_768 WHERE chunk_id = ?1", params![id])?;
                tx.execute("DELETE FROM memory_chunks WHERE id = ?1", params![id])?;
                tx.commit()?;
                Ok(())
            })
        })
        .await
    }

    async fn delete_all(&self) -> Result<u64> {
        let db = self.0.clone();
        blocking(move || {
            db.with_write(|conn| {
                let tx = conn.unchecked_transaction()?;
                tx.execute("DELETE FROM embeddings", [])?;
                tx.execute("DELETE FROM embeddings_768", [])?;
                let deleted = tx.execute("DELETE FROM memory_chunks", [])?;
                tx.commit()?;
                Ok(deleted as u64)
            })
        })
        .await
    }

    async fn get_stats(&self) -> Result<MemoryStats> {
        let db = self.0.clone();
        blocking(move || {
            db.with_read(|conn| {
                let total_chunks: i64 =
                    conn.query_row("SELECT COUNT(*) FROM memory_chunks", [], |row| row.get(0))?;
                let total_timeline_events: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM timeline_events",
                    [],
                    |row| row.get(0),
                )?;
                let oldest: Option<String> = conn.query_row(
                    "SELECT MIN(timestamp) FROM memory_chunks",
                    [],
                    |row| row.get(0),
                )?;
                let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
                let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;

                #[allow(clippy::cast_precision_loss)]
                let database_size_mb = (page_count * page_size) as f64 / (1024.0 * 1024.0);
                Ok(MemoryStats {
                    total_chunks: u64::try_from(total_chunks).unwrap_or(0),
                    total_timeline_events: u64::try_from(total_timeline_events).unwrap_or(0),
                    database_size_mb,
                    oldest_record: oldest.as_deref().map(parse_timestamp).transpose()?,
                })
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// TimelineRepository
// ---------------------------------------------------------------------------

pub struct SqliteTimelineRepository(pub Arc<Database>);

#[async_trait]
impl TimelineRepository for SqliteTimelineRepository {
    async fn insert_event(&self, event: &TimelineEvent) -> Result<()> {
        let db = self.0.clone();
        let event = event.clone();
        blocking(move || {
            db.with_write(|conn| {
                conn.execute(
                    "INSERT INTO timeline_events
                        (id, timestamp, event_type, summary, application, window_title,
                         duration_ms, context_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        event.id.to_string(),
                        event.timestamp.to_rfc3339(),
                        event.event_type.as_str(),
                        event.summary,
                        event.application,
                        event.window_title,
                        event.duration_ms,
                        event.context_id.map(|id| id.to_string()),
                    ],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn get_range(
        &self,
        range: TimeRange,
        pagination: Pagination,
    ) -> Result<Page<TimelineEvent>> {
        let db = self.0.clone();
        let start = range.start.to_rfc3339();
        let end = range.end.to_rfc3339();
        blocking(move || {
            db.with_read(|conn| {
                let total: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM timeline_events WHERE timestamp BETWEEN ?1 AND ?2",
                    params![start, end],
                    |row| row.get(0),
                )?;

                let mut stmt = conn.prepare(
                    "SELECT id, timestamp, event_type, summary, application, window_title, \
                            duration_ms, context_id \
                     FROM timeline_events \
                     WHERE timestamp BETWEEN ?1 AND ?2 \
                     ORDER BY timestamp DESC \
                     LIMIT ?3 OFFSET ?4",
                )?;
                let rows = stmt
                    .query_map(
                        params![start, end, pagination.limit, pagination.offset],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, String>(4)?,
                                row.get::<_, String>(5)?,
                                row.get::<_, Option<i64>>(6)?,
                                row.get::<_, Option<String>>(7)?,
                            ))
                        },
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()?;

                let items = rows
                    .into_iter()
                    .map(|r| -> Result<TimelineEvent> {
                        Ok(TimelineEvent {
                            id: parse_uuid(&r.0)?,
                            timestamp: parse_timestamp(&r.1)?,
                            event_type: r
                                .2
                                .parse::<EventType>()
                                .map_err(ContexaError::Conversion)?,
                            summary: r.3,
                            application: r.4,
                            window_title: r.5,
                            duration_ms: r.6,
                            context_id: r.7.as_deref().map(parse_uuid).transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;

                Ok(Page { items, total })
            })
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// McpRepository
// ---------------------------------------------------------------------------

pub struct SqliteMcpRepository(pub Arc<Database>);

#[async_trait]
impl McpRepository for SqliteMcpRepository {
    async fn create_token(&self, label: &str, token_hash: &str) -> Result<String> {
        let db = self.0.clone();
        let label = label.to_string();
        let token_hash = token_hash.to_string();
        let id = Uuid::new_v4().to_string();
        let id_for_insert = id.clone();
        // `created_at` inserted explicitly as RFC3339 (not the column's
        // SQLite-native `datetime('now')` default) — `parse_timestamp` below
        // expects RFC3339, same convention as every other table in this file.
        let created_at = Utc::now().to_rfc3339();
        blocking(move || {
            db.with_write(|conn| {
                conn.execute(
                    "INSERT INTO mcp_tokens (id, token_hash, label, created_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![id_for_insert, token_hash, label, created_at],
                )?;
                Ok(())
            })
        })
        .await?;
        Ok(id)
    }

    async fn find_active_tokens(&self) -> Result<Vec<TokenInfo>> {
        let db = self.0.clone();
        blocking(move || {
            db.with_read(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, token_hash, label, created_at, last_used_at, revoked \
                     FROM mcp_tokens WHERE revoked = 0",
                )?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, i64>(5)?,
                        ))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;

                rows.into_iter()
                    .map(|(id, token_hash, label, created_at, last_used_at, revoked)| {
                        Ok(TokenInfo {
                            id,
                            token_hash,
                            label,
                            created_at: parse_timestamp(&created_at)?,
                            last_used_at: last_used_at.as_deref().map(parse_timestamp).transpose()?,
                            revoked: revoked != 0,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
        })
        .await
    }

    async fn touch_token(&self, id: &str) -> Result<()> {
        let db = self.0.clone();
        let id = id.to_string();
        let last_used_at = Utc::now().to_rfc3339();
        blocking(move || {
            db.with_write(|conn| {
                conn.execute(
                    "UPDATE mcp_tokens SET last_used_at = ?1 WHERE id = ?2",
                    params![last_used_at, id],
                )?;
                Ok(())
            })
        })
        .await
    }

    async fn revoke_token(&self, id: &str) -> Result<()> {
        let db = self.0.clone();
        let id = id.to_string();
        blocking(move || {
            db.with_write(|conn| {
                conn.execute("UPDATE mcp_tokens SET revoked = 1 WHERE id = ?1", params![id])?;
                Ok(())
            })
        })
        .await
    }

    async fn log_tool_call(&self, token_id: &str, tool_name: &str, request_summary: &str) -> Result<()> {
        let db = self.0.clone();
        let token_id = token_id.to_string();
        let tool_name = tool_name.to_string();
        let request_summary = request_summary.to_string();
        blocking(move || {
            db.with_write(|conn| {
                conn.execute(
                    "INSERT INTO mcp_audit_log (token_id, tool_name, request_summary) \
                     VALUES (?1, ?2, ?3)",
                    params![token_id, tool_name, request_summary],
                )?;
                Ok(())
            })
        })
        .await
    }
}
