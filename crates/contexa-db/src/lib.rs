//! Database layer — `SQLite` + sqlite-vec via rusqlite — see `docs/04_Database_Design.md`

mod database;
mod model;
mod repository;

pub use database::Database;
pub use model::{
    EventType, MemoryChunk, Page, Pagination, PurgeStats, ScoredChunk, TimeRange, TimelineEvent,
};
pub use repository::{
    ContextRepository, MemoryRepository, SqliteContextRepository, SqliteMemoryRepository,
    SqliteTimelineRepository, TimelineRepository,
};
