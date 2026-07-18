use rand::{rngs::StdRng, Rng, SeedableRng};
use rusqlite::{params, Connection};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn parse_arg_usize(args: &[String], name: &str, default: usize) -> usize {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn vec_to_blob_f32_le(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Runs the insert+search benchmark against a fresh DB at `db_path`.
/// If `key` is Some, applies `PRAGMA key` right after opening (SQLCipher path).
/// Returns (unlock/open ms, query p50 ms, query p95 ms, query p99 ms, db size bytes).
fn run(
    db_path: &Path,
    key: Option<&str>,
    sqlite_vec_path: &Path,
    vectors: usize,
    dims: usize,
    queries: usize,
    top_k: usize,
    seed: u64,
    cache_kb: i64,
    mem_security_off: bool,
) -> Result<(u128, u128, u128, u128, u64), Box<dyn std::error::Error>> {
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
    }

    let t_open = Instant::now();
    let conn = Connection::open(db_path)?;
    if let Some(k) = key {
        // SQLCipher: key must be set before any other statement touches the DB file.
        conn.pragma_update(None, "key", k)?;
        if mem_security_off {
            // Skips SQLCipher's default zeroing of freed memory pages. Read-only perf tuning
            // knob (docs mention this trades a defense-in-depth guarantee for throughput) —
            // being measured here, not adopted as a default.
            conn.pragma_update(None, "cipher_memory_security", "OFF")?;
        }
        // Force SQLCipher to actually read/write the header now, so unlock cost is measured here
        // rather than lazily on first real query.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    }
    let open_ms = t_open.elapsed().as_millis();

    // Negative cache_size means KB (SQLite docs). Sized to hold the whole DB's pages so a
    // full-table vec0 scan doesn't repeatedly evict-and-redecrypt across queries.
    conn.pragma_update(None, "cache_size", -cache_kb)?;

    unsafe { conn.load_extension_enable()? };
    unsafe { conn.load_extension(sqlite_vec_path, None)? };

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        r#"
CREATE TABLE items(
  id INTEGER PRIMARY KEY,
  created_at INTEGER NOT NULL
);
"#,
    )?;
    conn.execute(
        &format!("CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[{dims}]);"),
        [],
    )?;

    let mut rng = StdRng::seed_from_u64(seed);
    let mut next_id: usize = 1;
    while next_id <= vectors {
        let batch_end_id = (next_id + 9).min(vectors);
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt_items = tx.prepare("INSERT INTO items(id, created_at) VALUES (?, ?)")?;
            let mut stmt_vec = tx.prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)")?;
            for id in next_id..=batch_end_id {
                stmt_items.execute(params![id as i64, 0i64])?;
                let mut v = vec![0.0f32; dims];
                for x in &mut v {
                    *x = rng.gen_range(-1.0..=1.0);
                }
                stmt_vec.execute(params![id as i64, vec_to_blob_f32_le(&v)])?;
            }
        }
        tx.commit()?;
        next_id = batch_end_id + 1;
    }

    let mut lat_ms: Vec<u128> = Vec::with_capacity(queries);
    for _ in 0..queries {
        let mut q = vec![0.0f32; dims];
        for x in &mut q {
            *x = rng.gen_range(-1.0..=1.0);
        }
        let qblob = vec_to_blob_f32_le(&q);

        let t0 = Instant::now();
        let mut stmt = conn.prepare(
            "SELECT rowid, distance FROM vec_items WHERE embedding MATCH ? ORDER BY distance LIMIT ?",
        )?;
        let mut rows = stmt.query(params![qblob, top_k as i64])?;
        let mut got = 0usize;
        while let Some(_row) = rows.next()? {
            got += 1;
        }
        if got != top_k {
            return Err(format!("Query returned {got} rows, expected {top_k}").into());
        }
        lat_ms.push(t0.elapsed().as_millis());
    }
    lat_ms.sort_unstable();
    let p50 = percentile(&lat_ms, 0.50);
    let p95 = percentile(&lat_ms, 0.95);
    let p99 = percentile(&lat_ms, 0.99);

    let db_bytes = std::fs::metadata(db_path)?.len();

    Ok((open_ms, p50, p95, p99, db_bytes))
}

/// Second-run check: reopen the encrypted DB with the same key and confirm data is readable,
/// and that a wrong key fails (proves the encryption is actually doing something, not a no-op).
fn verify_reopen(db_path: &Path, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "key", key)?;
    let count: i64 = conn.query_row("SELECT count(*) FROM items", [], |r| r.get(0))?;
    if count == 0 {
        return Err("reopen with correct key returned 0 rows".into());
    }

    let conn_wrong = Connection::open(db_path)?;
    conn_wrong.pragma_update(None, "key", "wrong-passphrase")?;
    let wrong_result = conn_wrong.query_row("SELECT count(*) FROM items", [], |r: &rusqlite::Row| r.get::<_, i64>(0));
    if wrong_result.is_ok() {
        return Err("wrong key was able to read encrypted data — encryption not effective".into());
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let vectors = parse_arg_usize(&args, "--vectors", 1_000);
    let dims = parse_arg_usize(&args, "--dims", 384);
    let queries = parse_arg_usize(&args, "--queries", 50);
    let top_k = parse_arg_usize(&args, "--topk", 10);
    // Default cache big enough for a 50K x 384-dim DB (~76MB) to fit fully in SQLite's page
    // cache, so a full-table vec0 scan isn't repeatedly evicting-and-redecrypting pages.
    let cache_kb = parse_arg_usize(&args, "--cache-kb", 200_000) as i64;
    let mem_security_off = args.iter().any(|a| a == "--mem-security-off");

    let sqlite_vec_path = env::var("SQLITE_VEC_PATH")
        .map(PathBuf::from)
        .map_err(|_| "Missing SQLITE_VEC_PATH (path to sqlite-vec vec0 extension .dll)")?;

    println!("SP-09 SQLCipher + sqlite-vec: vectors={vectors}, dims={dims}, queries={queries}, topK={top_k}, cache_kb={cache_kb}, mem_security_off={mem_security_off}");
    println!("sqlite-vec extension: {}", sqlite_vec_path.display());

    let plain_path = PathBuf::from("sp09_plain.sqlite3");
    let (plain_open, plain_p50, plain_p95, plain_p99, plain_size) = run(
        &plain_path,
        None,
        &sqlite_vec_path,
        vectors,
        dims,
        queries,
        top_k,
        42,
        cache_kb,
        mem_security_off,
    )?;
    println!(
        "PLAIN   open={plain_open}ms  p50={plain_p50}ms  p95={plain_p95}ms  p99={plain_p99}ms  size={:.2}MB",
        plain_size as f64 / (1024.0 * 1024.0)
    );

    let enc_path = PathBuf::from("sp09_encrypted.sqlite3");
    let key = "sp09-spike-passphrase";
    let (enc_open, enc_p50, enc_p95, enc_p99, enc_size) = run(
        &enc_path,
        Some(key),
        &sqlite_vec_path,
        vectors,
        dims,
        queries,
        top_k,
        42,
        cache_kb,
        mem_security_off,
    )?;
    println!(
        "ENCRYPTED open={enc_open}ms  p50={enc_p50}ms  p95={enc_p95}ms  p99={enc_p99}ms  size={:.2}MB",
        enc_size as f64 / (1024.0 * 1024.0)
    );

    verify_reopen(&enc_path, key)?;
    println!("Reopen check: correct key reads data, wrong key rejected — OK");

    let delta_pct = if plain_p95 > 0 {
        ((enc_p95 as f64 - plain_p95 as f64) / plain_p95 as f64) * 100.0
    } else {
        0.0
    };
    println!("Search p95 delta vs plain: {delta_pct:.1}%");
    println!("Unlock-on-open (encrypted): {enc_open}ms (target < 100ms)");

    if enc_open > 100 {
        println!("WARNING: unlock time exceeds 100ms target");
    }
    if delta_pct > 50.0 {
        println!("WARNING: search p95 delta exceeds +50% target");
    }

    Ok(())
}
