//! Database layer — `SQLite` + sqlite-vec via rusqlite — see `docs/04_Database_Design.md`

mod database;
mod model;
mod repository;

pub use database::{default_path, Database};
pub use model::{
    EventType, MemoryChunk, MemoryStats, Page, Pagination, PurgeStats, ScoredChunk, TimeRange,
    TimelineEvent, TokenInfo,
};
pub use repository::{
    ContextRepository, McpRepository, MemoryRepository, SqliteContextRepository,
    SqliteMcpRepository, SqliteMemoryRepository, SqliteTimelineRepository, TimelineRepository,
};
