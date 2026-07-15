// SP-01: UIA text extraction coverage (docs/22 §3).
// For each running target app: find its top-level window, walk the UIA tree
// (depth <= 20, element cap), collect Name + Value + TextPattern text.
// Measures: text length, extraction time, heuristic confidence.
// Gate: >= 8/10 apps confidence >= 0.8; p95 extraction < 150 ms.
// Ground-truth character accuracy remains a manual step (documented in report).

use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;
use uiautomation::controls::ControlType;
use uiautomation::patterns::{UITextPattern, UIValuePattern};
use uiautomation::types::TreeScope;
use uiautomation::{UIAutomation, UIElement};

const TARGETS: [(&str, &[&str]); 10] = [
    ("Google Chrome", &["chrome.exe"]),
    ("VS Code", &["Code.exe"]),
    ("Microsoft Word", &["WINWORD.EXE"]),
    ("Notepad", &["Notepad.exe"]),
    ("Windows Terminal", &["WindowsTerminal.exe"]),
    ("Adobe Acrobat", &["Acrobat.exe"]),
    ("Slack", &["slack.exe"]),
    ("Figma", &["Figma.exe"]),
    ("Excel", &["EXCEL.EXE"]),
    ("Outlook", &["OUTLOOK.EXE", "olk.exe"]), // olk.exe = new Outlook (WebView2)
];

const MAX_DEPTH: usize = 20;
const MAX_ELEMENTS: usize = 2000;
// production stops once the context budget is filled — mirror that here
const ENOUGH_CHARS: usize = 2000;

struct Extraction {
    chars: usize,
    elements: usize,
    ms: u128,
    text_pattern_chars: usize,
    sample: String,
}

fn walk(
    auto: &UIAutomation,
    el: &UIElement,
    depth: usize,
    seen: &mut usize,
    out: &mut String,
    tp_chars: &mut usize,
) {
    if depth > MAX_DEPTH || *seen > MAX_ELEMENTS || out.len() >= ENOUGH_CHARS {
        return;
    }
    *seen += 1;

    if let Ok(name) = el.get_name() {
        if !name.is_empty() {
            out.push_str(&name);
            out.push('\n');
        }
    }
    // pattern queries are COM QueryInterface calls — only pay for text-bearing controls
    let ct = el.get_control_type().ok();
    if matches!(ct, Some(ControlType::Edit) | Some(ControlType::ComboBox) | Some(ControlType::Document)) {
        if let Ok(vp) = el.get_pattern::<UIValuePattern>() {
            if let Ok(v) = vp.get_value() {
                if !v.is_empty() {
                    out.push_str(&v);
                    out.push('\n');
                }
            }
        }
    }
    // TextPattern on document/edit controls — the high-value extraction path
    if matches!(ct, Some(ControlType::Document) | Some(ControlType::Edit)) {
        if let Ok(tp) = el.get_pattern::<UITextPattern>() {
            if let Ok(range) = tp.get_document_range() {
                if let Ok(text) = range.get_text(65536) {
                    *tp_chars += text.len();
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
    }

    if let Ok(walker) = auto.create_tree_walker() {
        if let Ok(mut child) = walker.get_first_child(el) {
            loop {
                walk(auto, &child, depth + 1, seen, out, tp_chars);
                match walker.get_next_sibling(&child) {
                    Ok(next) => child = next,
                    Err(_) => break,
                }
                if *seen > MAX_ELEMENTS {
                    break;
                }
            }
        }
    }
}

/// All visible top-level windows as (hwnd, pid) — production-faithful path
/// (production tracks HWNDs, then ElementFromHandle).
fn visible_windows() -> Vec<(windows::Win32::Foundation::HWND, u32)> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
    };
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<(HWND, u32)>) };
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if IsWindowVisible(hwnd).as_bool() && GetWindowTextLengthW(hwnd) > 0 {
                out.push((hwnd, pid));
            }
        }
        true.into()
    }
    let mut wins: Vec<(HWND, u32)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut wins as *mut _ as isize));
    }
    wins
}

fn extract_for_pid(auto: &UIAutomation, pids: &[u32]) -> Option<Extraction> {
    let (hwnd, _) = visible_windows().into_iter().find(|(_, pid)| pids.contains(pid))?;
    let win = auto
        .element_from_handle(uiautomation::types::Handle::from(hwnd))
        .ok()?;

    let t = Instant::now();
    let mut out = String::new();
    let mut seen = 0usize;
    let mut tp_chars = 0usize;
    walk(auto, &win, 0, &mut seen, &mut out, &mut tp_chars);
    let ms = t.elapsed().as_millis();

    let sample: String = out.chars().take(160).collect::<String>().replace('\n', " | ");
    Some(Extraction { chars: out.len(), elements: seen, ms, text_pattern_chars: tp_chars, sample })
}

/// Heuristic confidence: 1.0 = rich text via TextPattern or large harvest;
/// 0.85 = solid text without TextPattern; 0.5 = fragments; 0.0 = nothing.
fn confidence(e: &Extraction) -> f32 {
    if e.text_pattern_chars > 50 || e.chars > 1000 {
        1.0
    } else if e.chars > 300 {
        0.85
    } else if e.chars > 50 {
        0.5
    } else {
        0.0
    }
}

fn main() -> Result<()> {
    let auto = UIAutomation::new()?;
    let sys = sysinfo::System::new_all();

    // process name (lowercase) -> pids
    let mut by_name: HashMap<String, Vec<u32>> = HashMap::new();
    for (pid, proc_) in sys.processes() {
        by_name
            .entry(proc_.name().to_string_lossy().to_lowercase())
            .or_default()
            .push(pid.as_u32());
    }

    let mut times: Vec<u128> = Vec::new();
    let mut pass = 0;
    let mut tested = 0;
    println!("{:<18} {:>8} {:>9} {:>7} {:>6}  {}", "app", "running", "chars", "ms", "conf", "sample");
    for (label, processes) in TARGETS {
        let pids: Vec<u32> = processes
            .iter()
            .flat_map(|p| by_name.get(&p.to_lowercase()).cloned().unwrap_or_default())
            .collect();
        if pids.is_empty() {
            println!("{:<18} {:>8} {:>9} {:>7} {:>6}", label, "no", "-", "-", "-");
            continue;
        }
        tested += 1;
        match extract_for_pid(&auto, &pids) {
            Some(e) => {
                let conf = confidence(&e);
                if conf >= 0.8 {
                    pass += 1;
                }
                times.push(e.ms);
                println!(
                    "{:<18} {:>8} {:>9} {:>7} {:>6.2}  {}",
                    label, "yes", e.chars, e.ms, conf,
                    &e.sample[..e.sample.len().min(80)]
                );
            }
            None => println!("{:<18} {:>8} {:>9} {:>7} {:>6}", label, "yes", "0", "-", "0.00 (no window)"),
        }
    }

    times.sort();
    let p95 = times.get((times.len() as f64 * 0.95) as usize).or(times.last());
    println!("\ntested {tested}/10 running apps; confidence>=0.8: {pass}/{tested}");
    if let Some(p95) = p95 {
        println!("extraction p95: {} ms [target < 150 ms]", p95);
    }
    println!("\nNote: apps not running were skipped — start them and re-run for full coverage.");
    Ok(())
}
