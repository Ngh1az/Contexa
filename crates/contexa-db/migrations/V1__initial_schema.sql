-- Initial v1.0 schema — docs/04_Database_Design.md §5.1-5.9.
-- v1.1 tables (meta_memories, entities, chunk_entities, work_threads,
-- encryption_meta) are deferred to a later migration per docs/04 §5.10.

CREATE TABLE context_snapshots (
    id              TEXT PRIMARY KEY NOT NULL,
    timestamp       TEXT NOT NULL,
    window_title    TEXT NOT NULL,
    process_name    TEXT NOT NULL,
    process_id      INTEGER NOT NULL,
    hwnd            INTEGER,
    url             TEXT,
    document_path   TEXT,
    visible_text    TEXT,
    selected_text   TEXT,
    metadata_json   TEXT DEFAULT '{}',
    language        TEXT,
    capture_method  TEXT NOT NULL CHECK(capture_method IN ('uia', 'ocr', 'hybrid')),
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_context_timestamp ON context_snapshots(timestamp);
CREATE INDEX idx_context_process ON context_snapshots(process_name);
CREATE INDEX idx_context_url ON context_snapshots(url) WHERE url IS NOT NULL;

CREATE TABLE timeline_events (
    id              TEXT PRIMARY KEY NOT NULL,
    timestamp       TEXT NOT NULL,
    event_type      TEXT NOT NULL CHECK(event_type IN (
                        'context_change', 'app_switch', 'user_query', 'ai_response'
                    )),
    summary         TEXT NOT NULL,
    application     TEXT NOT NULL,
    window_title    TEXT NOT NULL,
    duration_ms     INTEGER,
    context_id      TEXT REFERENCES context_snapshots(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_timeline_timestamp ON timeline_events(timestamp);
CREATE INDEX idx_timeline_type ON timeline_events(event_type);
CREATE INDEX idx_timeline_app ON timeline_events(application);

CREATE TABLE memory_chunks (
    id              TEXT PRIMARY KEY NOT NULL,
    context_id      TEXT REFERENCES context_snapshots(id) ON DELETE CASCADE,
    content         TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    timestamp       TEXT NOT NULL,
    application     TEXT NOT NULL,
    metadata_json   TEXT DEFAULT '{}',
    token_count     INTEGER DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_memory_hash ON memory_chunks(content_hash);
CREATE INDEX idx_memory_timestamp ON memory_chunks(timestamp);
CREATE INDEX idx_memory_app ON memory_chunks(application);

-- Load sqlite-vec extension at connection init (rusqlite — ADR/0010).
-- Default: all-MiniLM-L6-v2 (384-dim) via fastembed. See ADR/0006.
CREATE VIRTUAL TABLE embeddings USING vec0(
    chunk_id    TEXT PRIMARY KEY,
    embedding   FLOAT[384]
);

-- Quality mode: nomic-embed-text (768-dim) via Ollama
CREATE VIRTUAL TABLE embeddings_768 USING vec0(
    chunk_id    TEXT PRIMARY KEY,
    embedding   FLOAT[768]
);

CREATE TABLE embedding_meta (
    chunk_id        TEXT PRIMARY KEY REFERENCES memory_chunks(id) ON DELETE CASCADE,
    model           TEXT NOT NULL,
    dimensions      INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE user_settings (
    key             TEXT PRIMARY KEY NOT NULL,
    value_json      TEXT NOT NULL,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE user_queries (
    id              TEXT PRIMARY KEY NOT NULL,
    timestamp       TEXT NOT NULL,
    action          TEXT NOT NULL,
    query           TEXT,
    context_id      TEXT REFERENCES context_snapshots(id),
    latency_ms      INTEGER,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE ai_responses (
    id              TEXT PRIMARY KEY NOT NULL,
    query_id        TEXT NOT NULL REFERENCES user_queries(id) ON DELETE CASCADE,
    content         TEXT NOT NULL,
    model           TEXT NOT NULL,
    token_count     INTEGER,
    timestamp       TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_queries_timestamp ON user_queries(timestamp);

CREATE TABLE mcp_tokens (
    id              TEXT PRIMARY KEY NOT NULL,
    token_hash      TEXT NOT NULL UNIQUE,
    label           TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at    TEXT,
    revoked         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE mcp_audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    token_id        TEXT REFERENCES mcp_tokens(id),
    tool_name       TEXT NOT NULL,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    request_summary TEXT
);

CREATE INDEX idx_audit_timestamp ON mcp_audit_log(timestamp);

CREATE TABLE exclusions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    exclusion_type  TEXT NOT NULL CHECK(exclusion_type IN ('app', 'url', 'window_title')),
    pattern         TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_exclusion_pattern ON exclusions(exclusion_type, pattern);

-- Kept for schema parity with docs/04 §5.9. refinery tracks applied
-- migrations itself (its own internal table) — this table is not written to
-- by application code.
CREATE TABLE schema_version (
    version         INTEGER PRIMARY KEY,
    applied_at      TEXT NOT NULL DEFAULT (datetime('now')),
    description     TEXT
);
