use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

use contexa_context::{ContexaContextEngine, ContextEngine};
use contexa_core::ContextSnapshot;
use contexa_vision::{ContexaVisionEngine, VisionEngine};

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

/// docs/03 §5.2 `get_current_context` — `None` until the first snapshot has
/// been captured.
// Tauri's `#[tauri::command]` codegen requires `State<T>` by value; a
// reference isn't a valid command parameter type.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
fn get_current_context(context: tauri::State<Arc<ContexaContextEngine>>) -> Option<ContextSnapshot> {
    context.get_current()
}

// App entry point: an unrecoverable startup failure (e.g. WebView2 missing)
// should panic, not propagate — there's no caller to handle it.
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_current_context])
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

            // Vision Engine's capture thread feeds `rx`; nothing consumed it
            // until now (docs/14 §5.1, §5.2 — M1 wiring).
            let (vision, rx) = ContexaVisionEngine::new(Vec::new());
            vision.start()?;
            let context = Arc::new(ContexaContextEngine::with_builtin_enrichers());
            // Real UIA/clipboard selection tracking is opt-in (docs/06 §5.5)
            // — safe to enable here since this is the real app, not `cargo test`.
            context.enable_selection_tracking();

            let context_for_thread = Arc::clone(&context);
            std::thread::Builder::new()
                .name("contexa-context-consumer".to_string())
                .spawn(move || {
                    // Exits once `vision`'s Drop disconnects the channel (app exit).
                    while let Ok(result) = rx.recv() {
                        let _ = context_for_thread.process_vision_result(result);
                    }
                })?;

            app.manage(vision);
            app.manage(context);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
