//! SP-03: OCR Fallback Latency — `docs/22_Technical_Spike_Plan.md` §5.
//!
//! Acrobat/Slack/Figma (the spec's target apps) aren't installed on this
//! machine — the same gap SP-01 already recorded (see its `report.md`).
//! Uses the real production pipeline end-to-end
//! (`contexa_vision::FrameCapturer` + `contexa_vision::OcrEngine::ocr_region`,
//! the same code `crates/contexa-vision/src/engine.rs`'s `ocr_region` trait
//! method calls), so results measure exactly what the shipped code does.
//!
//! Usage:
//! - No args: captures whatever window currently has OS focus.
//! - `--hwnd <isize>`: captures a specific window by handle instead —
//!   avoids stealing focus from whatever the user is actually doing.
//!   Find a handle without touching focus via PowerShell:
//!   `(Get-Process notepad).MainWindowHandle.ToInt64()`.
//! - `--ground-truth <path>`: compares recognized text against a known
//!   text file (normalized Levenshtein similarity) and asserts > 90%
//!   accuracy (docs/22 §5 pass criterion).

use std::time::Instant;

use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

use contexa_vision::{FrameCapturer, OcrEngine, Region, WindowMonitor};

const ITERATIONS: usize = 10;
const TARGET_MS: u128 = 500; // docs/22 §5 pass criterion: "Single region OCR | < 500 ms"
const TARGET_ACCURACY: f64 = 0.90; // docs/22 §5: "Hybrid accuracy (UIA fail apps) | > 90%"
const TARGET_CPU_PERCENT: f64 = 15.0; // docs/22 §5: "CPU spike during OCR | < 15% for < 1 second"

struct Args {
    hwnd: Option<isize>,
    ground_truth: Option<String>,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut hwnd = None;
    let mut ground_truth = None;
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--hwnd" if i + 1 < argv.len() => {
                hwnd = argv[i + 1].parse::<isize>().ok();
                i += 2;
            }
            "--ground-truth" if i + 1 < argv.len() => {
                ground_truth = std::fs::read_to_string(&argv[i + 1]).ok();
                i += 2;
            }
            _ => i += 1,
        }
    }
    Args { hwnd, ground_truth }
}

fn main() -> anyhow::Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;

    let result = run();

    unsafe { CoUninitialize() };
    result
}

fn run() -> anyhow::Result<()> {
    let args = parse_args();

    let window = match args.hwnd {
        Some(hwnd) => WindowMonitor::window_info_for(hwnd)
            .ok_or_else(|| anyhow::anyhow!("no window found for hwnd {hwnd}"))?,
        None => WindowMonitor::current_window().ok_or_else(|| anyhow::anyhow!("no foreground window"))?,
    };
    println!("target window: {:?} ({})", window.title, window.process_name);

    // Uses the real production OCR path (contexa_vision::OcrEngine::ocr_region,
    // wired up after this spike's first pass) rather than hand-rolling a
    // second bitmap conversion here.
    let capturer = FrameCapturer::new();
    let ocr = OcrEngine::new();
    let mut latencies_ms = Vec::with_capacity(ITERATIONS);
    let mut last_text = String::new();

    let cpu_start = process_cpu_time_100ns();
    let wall_start = Instant::now();

    for i in 0..ITERATIONS {
        let frame = capturer
            .capture_window(window.hwnd)
            .map_err(|e| anyhow::anyhow!("capture failed: {e}"))?;

        // Crop out likely chrome (menu/tab bar, status bar, edge margins)
        // when a ground-truth comparison is requested — a full-window
        // capture legitimately includes real UI text, which isn't an OCR
        // accuracy error, just the wrong comparison. Production usage
        // always crops to a content region (docs/22 §5's whole premise).
        let region = if args.ground_truth.is_some() {
            let top = frame.height * 18 / 100;
            let bottom = frame.height * 8 / 100;
            let margin = frame.width * 3 / 100;
            Region {
                x: margin,
                y: top,
                width: frame.width.saturating_sub(2 * margin),
                height: frame.height.saturating_sub(top + bottom),
            }
        } else {
            Region {
                x: 0,
                y: 0,
                width: frame.width,
                height: frame.height,
            }
        };

        let started = Instant::now();
        let ocr_result = ocr
            .ocr_region(&frame, &region)
            .map_err(|e| anyhow::anyhow!("ocr failed: {e}"))?;
        let elapsed = started.elapsed();

        last_text = ocr_result.text;
        latencies_ms.push(elapsed.as_millis());
        println!(
            "iteration {i}: {} ms, {} chars",
            elapsed.as_millis(),
            last_text.len()
        );
    }

    let cpu_delta_100ns = process_cpu_time_100ns().saturating_sub(cpu_start);
    let wall_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
    let cpu_ms = cpu_delta_100ns as f64 / 10_000.0;
    let cores = std::thread::available_parallelism().map(std::num::NonZero::get).unwrap_or(1) as f64;
    // Task-manager style: total CPU-seconds spent, spread across all cores
    // and the measured wall-clock window (matches SP-02's reporting convention).
    let cpu_percent_machine = (cpu_ms / wall_ms / cores) * 100.0;

    latencies_ms.sort_unstable();
    let p50 = latencies_ms[latencies_ms.len() / 2];
    let p95_index = (latencies_ms.len() * 95 / 100).min(latencies_ms.len() - 1);
    let p95 = latencies_ms[p95_index];

    println!("\n--- SP-03 results ---");
    println!("p50: {p50} ms, p95: {p95} ms (target < {TARGET_MS} ms)");
    println!(
        "cpu: {cpu_percent_machine:.2}% of machine ({cores:.0} cores) over {ITERATIONS} calls, {wall_ms:.0}ms wall time (target < {TARGET_CPU_PERCENT:.0}%)"
    );
    println!(
        "recognized text:\n{}",
        last_text.chars().take(500).collect::<String>()
    );

    if let Some(truth) = &args.ground_truth {
        let accuracy = normalized_similarity(truth, &last_text);
        println!(
            "\nground-truth accuracy: {:.1}% (target > {:.0}%)",
            accuracy * 100.0,
            TARGET_ACCURACY * 100.0
        );
        assert!(
            accuracy > TARGET_ACCURACY,
            "accuracy {:.1}% is below the {:.0}% target",
            accuracy * 100.0,
            TARGET_ACCURACY * 100.0
        );
    }

    assert!(
        p95 < TARGET_MS,
        "p95 OCR latency {p95}ms exceeds the {TARGET_MS}ms target"
    );
    assert!(
        cpu_percent_machine < TARGET_CPU_PERCENT,
        "CPU usage {cpu_percent_machine:.2}% exceeds the {TARGET_CPU_PERCENT:.0}% target"
    );

    Ok(())
}

fn process_cpu_time_100ns() -> u64 {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe {
        // Best-effort: if this fails, treat elapsed CPU time as zero rather
        // than erroring the whole spike over a diagnostics-only measurement.
        let _ = GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user);
    }
    filetime_to_u64(kernel) + filetime_to_u64(user)
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

/// Character-level similarity via normalized Levenshtein distance, after
/// lowercasing and collapsing whitespace (OCR line-wrapping/spacing differs
/// from the source text's exact formatting, which isn't an accuracy error).
fn normalized_similarity(expected: &str, actual: &str) -> f64 {
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let a = normalize(expected);
    let b = normalize(actual);
    let max_len = a.chars().count().max(b.chars().count()).max(1);
    let distance = levenshtein(&a, &b);
    1.0 - (distance as f64 / max_len as f64)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_are_100_percent_similar() {
        assert!((normalized_similarity("hello world", "hello world") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn whitespace_and_case_differences_are_ignored() {
        assert!((normalized_similarity("Hello   World", "hello world") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn completely_different_strings_are_dissimilar() {
        assert!(normalized_similarity("abcdefgh", "zzzzzzzz") < 0.2);
    }
}
