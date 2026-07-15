use rand::{rngs::StdRng, Rng, SeedableRng};
use rusqlite::{params, Connection};
use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn parse_arg_u64(args: &[String], name: &str, default: u64) -> u64 {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_arg_usize(args: &[String], name: &str, default: usize) -> usize {
    parse_arg_u64(args, name, default as u64) as usize
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let vectors = parse_arg_usize(&args, "--vectors", 50_000);
    let dims = parse_arg_usize(&args, "--dims", 384);
    let queries = parse_arg_usize(&args, "--queries", 100);
    let top_k = parse_arg_usize(&args, "--topk", 10);

    let sqlite_vec_path = env::var("SQLITE_VEC_PATH")
        .map(PathBuf::from)
        .map_err(|_| "Missing SQLITE_VEC_PATH (path to sqlite-vec vec0 extension .dll)")?;

    let db_path = PathBuf::from("sp04.sqlite3");
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    // ponytail: use on-disk db so size is measurable; upgrade path is using a temp dir per run.
    let conn = Connection::open(&db_path)?;

    // Required for extension loading on Windows in many configs.
    unsafe { conn.load_extension_enable()? };
    unsafe { conn.load_extension(&sqlite_vec_path, None)? };

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        r#"
CREATE TABLE items(
  id INTEGER PRIMARY KEY,
  created_at INTEGER NOT NULL
);
"#,
    )?;

    // vec0 schema: store f32 vector as little-endian bytes.
    // NOTE: sqlite-vec expects vectors as blobs; the function used for distance is `vec_distance_cosine`.
    conn.execute(
        &format!(
            "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[{dims}]);"
        ),
        [],
    )?;

    let mut rng = StdRng::seed_from_u64(42);

    println!(
        "SP-04 sqlite-vec baseline: vectors={}, dims={}, queries={}, topK={}",
        vectors, dims, queries, top_k
    );
    println!("DB: {}", db_path.display());
    println!("sqlite-vec extension: {}", sqlite_vec_path.display());

    let t_insert_start = Instant::now();
    let mut insert_batch_10_ms: Vec<u128> = Vec::with_capacity((vectors + 9) / 10);
    {
        let mut next_id: usize = 1;
        while next_id <= vectors {
            let batch_start_id = next_id;
            let batch_end_id = (next_id + 9).min(vectors);

            let t0 = Instant::now();
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt_items =
                    tx.prepare("INSERT INTO items(id, created_at) VALUES (?, ?)")?;
                let mut stmt_vec =
                    tx.prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)")?;

                for id in batch_start_id..=batch_end_id {
                    stmt_items.execute(params![id as i64, 0i64])?;

                    let mut v = vec![0.0f32; dims];
                    for x in &mut v {
                        *x = rng.gen_range(-1.0..=1.0);
                    }
                    let blob = vec_to_blob_f32_le(&v);
                    stmt_vec.execute(params![id as i64, blob])?;
                }
            }
            tx.commit()?;
            insert_batch_10_ms.push(t0.elapsed().as_millis());

            if batch_end_id % 10_000 == 0 {
                println!("Inserted {} vectors...", batch_end_id);
            }

            next_id = batch_end_id + 1;
        }
    }
    let insert_elapsed = t_insert_start.elapsed();
    println!("Insert total: {} ms", insert_elapsed.as_millis());

    insert_batch_10_ms.sort_unstable();
    let b10_p50 = percentile(&insert_batch_10_ms, 0.50);
    let b10_p95 = percentile(&insert_batch_10_ms, 0.95);
    println!("Insert batch(10) ms: p50={b10_p50}, p95={b10_p95}");

    // Query benchmark
    let mut lat_ms: Vec<u128> = Vec::with_capacity(queries);
    for _ in 0..queries {
        let mut q = vec![0.0f32; dims];
        for x in &mut q {
            *x = rng.gen_range(-1.0..=1.0);
        }
        let qblob = vec_to_blob_f32_le(&q);

        // Use vec0's KNN query form (MATCH + ORDER BY distance) to avoid a full scan.
        // See sqlite-vec README "KNN style query".
        let t0 = Instant::now();
        let mut stmt = conn.prepare(
            "SELECT rowid, distance
             FROM vec_items
             WHERE embedding MATCH ?
             ORDER BY distance
             LIMIT ?",
        )?;
        let mut rows = stmt.query(params![qblob, top_k as i64])?;
        let mut got = 0usize;
        while let Some(_row) = rows.next()? {
            got += 1;
        }
        let dt = t0.elapsed();
        if got != top_k {
            return Err(format!("Query returned {got} rows, expected {top_k}").into());
        }
        lat_ms.push(dt.as_millis());
    }

    lat_ms.sort_unstable();
    let p50 = percentile(&lat_ms, 0.50);
    let p95 = percentile(&lat_ms, 0.95);
    let p99 = percentile(&lat_ms, 0.99);
    println!("Query latency ms: p50={p50}, p95={p95}, p99={p99}");

    // Approx DB size
    let db_bytes = std::fs::metadata(&db_path)?.len();
    println!("DB size: {:.2} MB", db_bytes as f64 / (1024.0 * 1024.0));

    // Also report mean to sanity check.
    let mean: f64 = lat_ms.iter().copied().map(|x| x as f64).sum::<f64>() / lat_ms.len() as f64;
    println!("Query mean ms: {:.2}", mean);

    // Simple "runnable check": ensure we at least ran queries and got plausible timings.
    let max_ok = Duration::from_secs(5);
    if Duration::from_millis(p95 as u64) > max_ok {
        println!("WARNING: p95 > 5000ms (unexpectedly slow). Check extension path and build mode.");
    }

    Ok(())
}

