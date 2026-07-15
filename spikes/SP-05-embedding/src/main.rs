// SP-05: Embedding model selection — fastembed all-MiniLM-L6-v2 (384-dim default path).
// Measures: MRR@10 over 20 queries vs 100 chunks, batch embed latency (10 chunks), model memory.
// Gate (docs/22 §7): MRR@10 > 0.7, batch embed < 0.5s, memory < 200 MB.

use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::time::Instant;

/// 20 (query, relevant chunk) pairs simulating desktop context recall.
/// Query wording deliberately differs from chunk wording.
const PAIRS: [(&str, &str); 20] = [
    ("how do I fix the OAuth token refresh bug", "The refresh_token endpoint returns 401 because the client secret rotated last Tuesday. Update the secret in the vault and redeploy the auth service."),
    ("what did the quarterly sales report say", "Q2 revenue reached $4.2M, up 18% YoY. Enterprise segment drove growth while SMB churn increased to 3.1% monthly."),
    ("rust lifetime error in the parser module", "error[E0597]: `input` does not live long enough — the Tokenizer borrows the source string, so the AST must not outlive the buffer passed to parse()."),
    ("when is the design review meeting", "Design review for the overlay component is scheduled Thursday 2pm in the Aurora room, covering dark theme tokens and focus states."),
    ("customer complaint about slow search", "Ticket #4821: user reports timeline search taking over 8 seconds after upgrading to 90 days retention. Suspect missing index on timestamp."),
    ("how to configure the CI pipeline cache", "GitHub Actions: use actions/cache with key cargo-${{ hashFiles('Cargo.lock') }} and path ~/.cargo/registry to cut build time roughly in half."),
    ("what database migration renamed the events table", "Migration V7__rename_activity.sql renames events to activity_events and adds a composite index on (session_id, created_at)."),
    ("recipe shared in the team channel", "Maria posted her ramen broth recipe: simmer pork bones 12 hours, add kombu in the last 30 minutes, season with shoyu tare."),
    ("flight booking for the berlin conference", "Booking confirmation LH1044: departure June 3rd 07:45 SGN, arrival BER 15:20 with one layover in Frankfurt, seat 23A."),
    ("kubernetes pod keeps restarting", "CrashLoopBackOff on context-worker pod: OOMKilled at 512Mi limit. Raise memory limit to 1Gi or reduce embedding batch size."),
    ("what did legal say about the privacy policy", "Legal review: the draft must disclose local storage duration, add a data deletion contact address, and clarify GDPR data-controller status before publishing."),
    ("performance numbers from the load test", "Load test with 200 concurrent users: p95 API latency 340ms, error rate 0.02%, database CPU peaked at 71% on the writer instance."),
    ("how does the retry logic handle rate limits", "The HTTP client retries 429 responses with exponential backoff starting at 250ms, doubling up to 8s, honoring the Retry-After header when present."),
    ("who approved the marketing budget increase", "Budget memo: An Nguyen approved raising the paid acquisition budget to $30K/month for Q3, contingent on CAC staying under $95."),
    ("typescript type error with the event emitter", "TS2345: Argument of type 'ContextEvent' is not assignable to 'SnapshotEvent' — the emitter map needs a discriminated union keyed by event name."),
    ("dentist appointment reminder", "Reminder: dental cleaning appointment Monday 9:30am at SmileCare clinic, District 3. Arrive 10 minutes early for insurance paperwork."),
    ("how to enable WAL mode in sqlite", "Set PRAGMA journal_mode=WAL and synchronous=NORMAL right after opening the connection; WAL allows concurrent readers during a single writer."),
    ("feedback from the beta user interview", "Beta interview notes: user loves the hotkey overlay but wants an easy pause button and clearer indication of which apps are being captured."),
    ("gpu driver crash while gaming", "Event log: nvlddmkm reset after TDR timeout during Cyberpunk session; suggests driver 551.23 rollback or disabling MPO via registry."),
    ("what is the plan for onboarding new engineers", "Onboarding plan: day 1 environment setup with the bootstrap script, week 1 shadow a senior on a starter ticket, week 2 own a small feature end to end."),
];

/// 80 distractor chunks — plausible desktop noise across the same broad domains.
fn distractors() -> Vec<String> {
    let templates: [&str; 20] = [
        "Meeting notes {i}: sprint planning moved velocity target to {n} points; retro highlighted flaky integration tests as the top annoyance.",
        "Email {i}: your package with order number 88-{n} has shipped and will arrive within 3 business days via GHN Express.",
        "Code review {i}: prefer iterator chains over index loops here, and extract the {n}-line match block into a helper for readability.",
        "Slack message {i}: lunch at the pho place at 12? Also someone left a laptop charger {n} in meeting room B.",
        "Documentation {i}: the config file supports {n} top-level keys; unknown keys are rejected at startup with a validation error.",
        "Browser article {i}: researchers demonstrate a {n}% improvement in battery density using solid-state electrolytes in lab conditions.",
        "Terminal output {i}: compiled 14{n} crates in 92.4s, warning: unused variable `snapshot` in context/assembler.rs line {n}.",
        "Calendar event {i}: 1:1 with manager rescheduled to Friday {n}pm due to conflicting all-hands meeting.",
        "Invoice {i}: cloud hosting bill for May totals ${n}4.90 including egress overage of 120GB on the backup bucket.",
        "Chat {i}: the staging environment will be down for {n0} minutes tonight for the postgres 16 upgrade, ping ops if blocked.",
        "News {i}: local weather forecast predicts {n} days of heavy rain; flooding possible in low-lying districts near the river.",
        "Jira ticket {i}: as a user I want keyboard navigation in the settings dialog so that mouse-free workflows are possible, {n} story points.",
        "README {i}: run npm install then npm run dev; the dev server listens on port 30{n} with hot reload enabled.",
        "Forum post {i}: has anyone benchmarked zstd vs lz4 for log compression? Seeing {n}x ratio but higher CPU on writes.",
        "Receipt {i}: coffee subscription renewal ${n}.99 monthly, next billing date August 1st, cancel anytime in account settings.",
        "Wiki {i}: incident postmortem template requires timeline, root cause, {n} action items with owners, and a lessons-learned section.",
        "Log {i}: connection pool exhausted after {n}00 concurrent requests; increase max_connections or add pgbouncer in transaction mode.",
        "Presentation {i}: slide {n} covers the competitive landscape quadrant with axes of privacy focus versus context depth.",
        "Message {i}: mom asked if you are coming home for the holidays; flights get expensive after week {n} of December.",
        "Tutorial {i}: chapter {n} explains ownership and borrowing with diagrams of stack frames and heap allocations.",
    ];
    let mut out = Vec::with_capacity(80);
    for i in 0..80 {
        let t = templates[i % 20];
        let s = t
            .replace("{i}", &format!("{}", i + 1))
            .replace("{n0}", &format!("{}", (i % 9 + 1) * 10))
            .replace("{n}", &format!("{}", i % 9 + 1));
        out.push(s);
    }
    out
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

fn mem_mb() -> f64 {
    memory_stats::memory_stats().map(|m| m.physical_mem as f64 / 1_048_576.0).unwrap_or(0.0)
}

fn main() -> Result<()> {
    let mem_before = mem_mb();
    let t_load = Instant::now();
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    )?;
    let load_ms = t_load.elapsed().as_millis();
    let mem_after_load = mem_mb();

    // Corpus: 20 relevant chunks (index 0..20) + 80 distractors (index 20..100)
    let mut chunks: Vec<String> = PAIRS.iter().map(|(_, c)| c.to_string()).collect();
    chunks.extend(distractors());
    assert_eq!(chunks.len(), 100);

    // Embed corpus (warm-up happens here too)
    let t_corpus = Instant::now();
    let chunk_embs = model.embed(chunks.clone(), None)?;
    let corpus_ms = t_corpus.elapsed().as_millis();

    // Batch embed latency: 10 chunks, 5 runs, take median
    let batch: Vec<String> = chunks[0..10].to_vec();
    let mut batch_times: Vec<u128> = (0..5)
        .map(|_| {
            let t = Instant::now();
            let _ = model.embed(batch.clone(), None).unwrap();
            t.elapsed().as_millis()
        })
        .collect();
    batch_times.sort();
    let batch_median_ms = batch_times[2];

    // Queries → MRR@10
    let queries: Vec<String> = PAIRS.iter().map(|(q, _)| q.to_string()).collect();
    let query_embs = model.embed(queries, None)?;

    let mut rr_sum = 0.0f64;
    let mut hits_at_1 = 0;
    for (qi, qe) in query_embs.iter().enumerate() {
        let mut scored: Vec<(usize, f32)> =
            chunk_embs.iter().enumerate().map(|(ci, ce)| (ci, cosine(qe, ce))).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let rank = scored.iter().position(|(ci, _)| *ci == qi).unwrap() + 1;
        if rank == 1 { hits_at_1 += 1; }
        if rank <= 10 { rr_sum += 1.0 / rank as f64; }
        println!("query {:2}: relevant chunk ranked #{}", qi + 1, rank);
    }
    let mrr = rr_sum / 20.0;
    let mem_peak = mem_mb();

    println!("\n=== SP-05 results (fastembed all-MiniLM-L6-v2, 384-dim) ===");
    println!("model load: {} ms", load_ms);
    println!("corpus embed (100 chunks): {} ms", corpus_ms);
    println!("batch embed 10 chunks (median of 5): {} ms  [target < 500 ms]", batch_median_ms);
    println!("MRR@10: {:.3}  [target > 0.7]", mrr);
    println!("hits@1: {}/20", hits_at_1);
    println!("memory before/after-load/peak: {:.0} / {:.0} / {:.0} MB  [model delta target < 200 MB]",
        mem_before, mem_after_load, mem_peak);

    // ponytail: single runnable check for the spike gate
    assert!(mrr > 0.7, "GATE FAIL: MRR@10 {:.3} <= 0.7", mrr);
    assert!(batch_median_ms < 500, "GATE FAIL: batch embed {} ms >= 500 ms", batch_median_ms);
    println!("\nGATE: PASS");
    Ok(())
}
