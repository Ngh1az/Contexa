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

// App entry point: an unrecoverable startup failure (e.g. WebView2 missing)
// should panic, not propagate — there's no caller to handle it.
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Preload: window exists from startup; stay hidden until hotkey.
            // Validated in SP-07 (open latency p50 5ms / p95 9ms).
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
