//! Connection management (WAL, sqlite-vec extension loading, migrations) —
//! `docs/04_Database_Design.md` §9, `ADR/0010-rusqlite-database-access.md`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use contexa_core::Result;

mod embedded {
    refinery::embed_migrations!("./migrations");
}

const READ_POOL_SIZE: usize = 4;
const DEFAULT_VEC_EXTENSION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vendor/sqlite-vec/vec0.dll"
);

/// `%APPDATA%\Contexa\contexa.db` — `docs/04_Database_Design.md` §"Database
/// file location". Shared by the desktop app and the separately-spawned MCP
/// server binary (`crates/contexa-mcp/src/bin/contexa_mcp_server.rs`) so
/// both open the same file — single source of truth for the path.
#[must_use]
pub fn default_path() -> PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(app_data).join("Contexa").join("contexa.db")
}

/// Single writer + round-robin read pool over one `SQLite` (WAL) file, with the
/// sqlite-vec extension loaded on every connection.
pub struct Database {
    write: Mutex<Connection>,
    read_pool: Vec<Mutex<Connection>>,
    next_reader: AtomicUsize,
}

impl Database {
    /// Opens (creating if needed) the database at `path`, loads the sqlite-vec
    /// extension on every connection, and runs migrations.
    ///
    /// # Errors
    /// Returns an error if the file can't be opened, the sqlite-vec extension
    /// can't be loaded, or a migration fails.
    pub fn open(path: &Path, vec_extension_path: Option<&Path>) -> Result<Self> {
        let ext_path = resolve_extension_path(vec_extension_path);

        let mut write_conn = Connection::open(path)?;
        load_vec_extension(&write_conn, &ext_path)?;
        apply_writer_pragmas(&write_conn)?;
        embedded::migrations::runner().run(&mut write_conn)?;

        let mut read_pool = Vec::with_capacity(READ_POOL_SIZE);
        for _ in 0..READ_POOL_SIZE {
            let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            load_vec_extension(&conn, &ext_path)?;
            apply_common_pragmas(&conn)?;
            read_pool.push(Mutex::new(conn));
        }

        Ok(Self {
            write: Mutex::new(write_conn),
            read_pool,
            next_reader: AtomicUsize::new(0),
        })
    }

    /// Runs `f` with the single write connection.
    ///
    /// # Errors
    /// Propagates whatever error `f` returns.
    pub fn with_write<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self
            .write
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&conn)
    }

    /// Runs `f` with the next read connection from the pool (round-robin).
    ///
    /// # Errors
    /// Propagates whatever error `f` returns.
    pub fn with_read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let idx = self.next_reader.fetch_add(1, Ordering::Relaxed) % self.read_pool.len();
        let conn = self.read_pool[idx]
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&conn)
    }
}

fn resolve_extension_path(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Ok(p) = std::env::var("SQLITE_VEC_PATH") {
        return PathBuf::from(p);
    }
    PathBuf::from(DEFAULT_VEC_EXTENSION)
}

fn load_vec_extension(conn: &Connection, ext_path: &Path) -> Result<()> {
    // SAFETY: loading the vendored sqlite-vec extension (vendor/sqlite-vec/vec0.dll,
    // the same binary validated in spikes/SP-04-sqlite-vec — see benchmarks/BASELINE.md)
    // at connection init, before any untrusted input reaches the connection.
    unsafe {
        conn.load_extension_enable()?;
        conn.load_extension(ext_path, None)?;
    }
    Ok(())
}

fn apply_writer_pragmas(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    apply_common_pragmas(conn)
}

fn apply_common_pragmas(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "cache_size", -64_000_i64)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "mmap_size", 268_435_456_i64)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}
