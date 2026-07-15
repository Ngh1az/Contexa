use std::time::Instant;

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

fn overlay(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "overlay window 'main' not found".into())
}

fn show_overlay(win: &WebviewWindow) -> Result<(), String> {
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn hide_overlay(win: &WebviewWindow) -> Result<(), String> {
    win.hide().map_err(|e| e.to_string())
}

fn toggle_overlay(app: &AppHandle) -> Result<(), String> {
    let win = overlay(app)?;
    if win.is_visible().map_err(|e| e.to_string())? {
        hide_overlay(&win)
    } else {
        show_overlay(&win)
    }
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Programmatic open latency: timer starts immediately before show+focus.
/// This models "hotkey already received → overlay visible" with a preloaded WebView.
#[tauri::command]
fn run_open_latency_bench(app: AppHandle, iterations: Option<u32>) -> Result<String, String> {
    let n = iterations.unwrap_or(100) as usize;
    let win = overlay(&app)?;

    // Ensure we start from hidden (preloaded) state.
    let _ = hide_overlay(&win);

    let mut open_ms: Vec<u128> = Vec::with_capacity(n);
    let mut focus_ms: Vec<u128> = Vec::with_capacity(n);

    for _ in 0..n {
        let t0 = Instant::now();
        win.show().map_err(|e| e.to_string())?;
        let shown = t0.elapsed().as_millis();
        win.set_focus().map_err(|e| e.to_string())?;
        let focused = t0.elapsed().as_millis();

        open_ms.push(shown);
        // ponytail: focus-steal proxy = time from show to focus call finishes (not OS focus steal API).
        focus_ms.push(focused.saturating_sub(shown));

        std::thread::sleep(std::time::Duration::from_millis(5));
        hide_overlay(&win)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    open_ms.sort_unstable();
    focus_ms.sort_unstable();

    let open_p50 = percentile(&open_ms, 0.50);
    let open_p95 = percentile(&open_ms, 0.95);
    let focus_p50 = percentile(&focus_ms, 0.50);
    let focus_p95 = percentile(&focus_ms, 0.95);

    let summary = format!(
        "iterations={n}\nopen_latency_ms: p50={open_p50}, p95={open_p95}\nfocus_steal_ms: p50={focus_p50}, p95={focus_p95}"
    );
    println!("{summary}");
    Ok(summary)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![run_open_latency_bench])
        .setup(|app| {
            // Preload: window exists from startup; stay hidden until hotkey.
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.hide();
            }

            #[cfg(desktop)]
            {
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["alt+space"])?
                        .with_handler(|app, shortcut, event| {
                            if event.state != ShortcutState::Pressed {
                                return;
                            }
                            if shortcut.matches(Modifiers::ALT, Code::Space) {
                                let _ = toggle_overlay(app);
                            }
                        })
                        .build(),
                )?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
