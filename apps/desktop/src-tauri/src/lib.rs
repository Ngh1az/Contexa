use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow, WindowEvent,
};
use uuid::Uuid;

use contexa_context::{ContexaContextEngine, ContextEngine};
use contexa_core::{ContextSnapshot, RequestAction, RequestPreferences, UserRequest};
use contexa_db::{
    Database, SqliteContextRepository, SqliteMemoryRepository, SqliteTimelineRepository,
};
use contexa_llm::{OllamaProvider, ProviderSelector};
use contexa_memory::{ContexaMemoryEngine, MemoryEngine};
use contexa_orchestrator::{AiOrchestrator, ContexaOrchestrator, PipelineConfig, PipelineManager};
use contexa_prompt::{ContexaPromptBuilder, PromptBuilder};
use contexa_search::{ContexaSearchEngine, SearchEngine};
use contexa_vision::{ContexaVisionEngine, VisionEngine};

// Owner's choice over ADR-0007's literal "llama3.2:3b or phi3:mini"
// recommendation — Qwen3 8B for better quality on capable hardware. No
// cloud LLM pre-configured either way; Settings UI (Phase 3, not built yet)
// will make this user-configurable.
const DEFAULT_OLLAMA_MODEL: &str = "qwen3:8b";

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

/// No global hotkey (docs/12 §5.3, §10 pivot): overlay opens only from the
/// tray — left-click the icon or "Open Overlay" in its menu. "Quit" here is
/// the only way to exit; the title-bar close button still just hides
/// (`intercept_close_to_hide`).
#[allow(clippy::expect_used)]
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open_overlay", "Open Overlay", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("bundle icon configured in tauri.conf.json").clone())
        .tooltip("Contexa")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_overlay" => {
                let _ = toggle_overlay(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let _ = toggle_overlay(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Native title bar now has a real close (X) button (docs/12 §5.3 pivot —
/// regular resizable window, not a frameless popup). Clicking it must hide,
/// not destroy: destroying would drop the preloaded webview (losing SP-07's
/// instant reopen) and, more importantly, background capture/memory ingest
/// (docs/16 §7.1) must keep running until the user quits from the tray, not
/// on every accidental close click.
fn intercept_close_to_hide(win: &WebviewWindow) {
    let win_for_close = win.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_for_close.hide();
        }
    });
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

/// docs/03 §5.2 `handle_request` request/response shape.
#[derive(Debug, serde::Deserialize)]
struct HandleRequestParams {
    action: String,
    query: Option<String>,
    target_lang: Option<String>,
    #[serde(default = "default_stream")]
    stream: bool,
}

fn default_stream() -> bool {
    true
}

#[derive(Debug, serde::Serialize)]
struct HandleRequestResponse {
    request_id: String,
    status: String,
    reason: Option<String>,
}

fn rejected(reason: impl Into<String>) -> HandleRequestResponse {
    HandleRequestResponse {
        request_id: String::new(),
        status: "rejected".to_string(),
        reason: Some(reason.into()),
    }
}

fn parse_action(action: &str, target_lang: Option<String>) -> Result<RequestAction, String> {
    match action {
        "chat" => Ok(RequestAction::Chat),
        "explain" => Ok(RequestAction::Explain),
        "summarize" => Ok(RequestAction::Summarize),
        "translate" => target_lang
            .map(|target_lang| RequestAction::Translate { target_lang })
            .ok_or_else(|| "target_lang is required for the translate action".to_string()),
        "search" => Ok(RequestAction::Search),
        "recall" => Ok(RequestAction::Recall),
        other => Err(format!("unknown action: {other}")),
    }
}

/// docs/03 §5.2 `handle_request`: accepts the request, returns a
/// `request_id` immediately, and streams the response via `ai-chunk`/
/// `ai-complete`/`ai-error` events (§5.3) — the caller doesn't block on the
/// LLM call.
#[tauri::command]
async fn handle_request(
    app: AppHandle,
    orchestrator: tauri::State<'_, Arc<ContexaOrchestrator>>,
    params: HandleRequestParams,
) -> Result<HandleRequestResponse, String> {
    let action = match parse_action(&params.action, params.target_lang) {
        Ok(action) => action,
        Err(reason) => return Ok(rejected(reason)),
    };

    let request = UserRequest {
        id: Uuid::new_v4(),
        action,
        query: params.query,
        context_override: None,
        preferences: RequestPreferences {
            stream: params.stream,
            ..RequestPreferences::default()
        },
    };

    let handle = match orchestrator.handle_request(request).await {
        Ok(handle) => handle,
        Err(e) => return Ok(rejected(e.to_string())),
    };

    let request_id = handle.id.clone();
    let orchestrator = Arc::clone(orchestrator.inner());
    let stream_app = app.clone();
    let stream_id = request_id.clone();
    tauri::async_runtime::spawn(async move {
        stream_response(&stream_app, &orchestrator, &stream_id).await;
    });

    Ok(HandleRequestResponse {
        request_id,
        status: "accepted".to_string(),
        reason: None,
    })
}

async fn stream_response(app: &AppHandle, orchestrator: &ContexaOrchestrator, request_id: &str) {
    let Some(mut stream) = orchestrator.take_stream(request_id).await else {
        let _ = app.emit(
            "ai-error",
            serde_json::json!({
                "request_id": request_id,
                "error": "request failed before producing a response stream",
                "code": "no_stream",
            }),
        );
        return;
    };

    let start = std::time::Instant::now();
    let mut chunk_count: u64 = 0;
    loop {
        match stream.recv().await {
            Some(Ok(content)) => {
                chunk_count += 1;
                let _ = app.emit(
                    "ai-chunk",
                    serde_json::json!({ "request_id": request_id, "content": content, "done": false }),
                );
            }
            Some(Err(e)) => {
                let _ = app.emit(
                    "ai-error",
                    serde_json::json!({ "request_id": request_id, "error": e.to_string(), "code": "provider_error" }),
                );
                return;
            }
            None => break,
        }
    }

    let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let _ = app.emit(
        "ai-complete",
        serde_json::json!({ "request_id": request_id, "total_tokens": chunk_count, "latency_ms": latency_ms }),
    );
}

/// docs/03 §5.2 `cancel_request`.
#[tauri::command]
async fn cancel_request(
    orchestrator: tauri::State<'_, Arc<ContexaOrchestrator>>,
    request_id: String,
) -> Result<(), String> {
    orchestrator
        .cancel_request(&request_id)
        .await
        .map_err(|e| e.to_string())
}

// App entry point: an unrecoverable startup failure (e.g. WebView2 missing)
// should panic, not propagate — there's no caller to handle it.
#[allow(clippy::expect_used, clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_current_context,
            handle_request,
            cancel_request
        ])
        .setup(|app| {
            // Preload: window exists from startup; stay hidden until opened
            // from the tray. Validated in SP-07 (open latency p50 5ms / p95 9ms).
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.hide();
                intercept_close_to_hide(&win);
            }

            build_tray(app.handle())?;

            // Vision Engine's capture thread feeds `rx`; nothing consumed it
            // until now (docs/14 §5.1, §5.2 — M1 wiring).
            // Default exclusions (docs/16 §6.2) until Settings UI (Phase 3)
            // lets the user add/remove their own on top of these.
            let (vision, rx) = ContexaVisionEngine::new(contexa_vision::ExclusionFilter::default_rules());
            vision.start()?;
            let vision = Arc::new(vision);
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

            // docs/04 §"Database file location".
            let db_path = contexa_db::default_path();
            if let Some(dir) = db_path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let db = Arc::new(Database::open(&db_path, None)?);
            let context_repo = Arc::new(SqliteContextRepository(Arc::clone(&db)));
            let memory_repo = Arc::new(SqliteMemoryRepository(Arc::clone(&db)));
            let timeline_repo = Arc::new(SqliteTimelineRepository(Arc::clone(&db)));

            // `ContexaMemoryEngine::new` spawns its periodic-flush task via
            // plain `tokio::spawn`, which panics ("no reactor running")
            // unless called from inside an active Tokio runtime context —
            // Tauri's `setup` closure runs synchronously outside one.
            // `block_on` enters Tauri's runtime for the duration of the call.
            let memory_engine: Arc<dyn MemoryEngine> =
                Arc::new(tauri::async_runtime::block_on(async {
                    ContexaMemoryEngine::new(context_repo, memory_repo, timeline_repo)
                })?);

            // Session Memory tier (docs/07 §5): every context update is
            // persisted, not just the ones the overlay explicitly asks for —
            // nothing consumed `context`'s broadcast until now.
            let mut memory_rx = context.subscribe();
            let memory_for_ingest = Arc::clone(&memory_engine);
            tauri::async_runtime::spawn(async move {
                loop {
                    match memory_rx.recv().await {
                        Ok(snapshot) => {
                            if let Err(e) = memory_for_ingest.ingest(&snapshot).await {
                                tracing::warn!(error = %e, "memory ingest failed");
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(
                                skipped,
                                "memory ingest consumer lagged; some context updates were dropped"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            let prompt_builder: Arc<dyn PromptBuilder> = Arc::new(ContexaPromptBuilder::default());
            let provider = ProviderSelector::new(Box::new(OllamaProvider::new(DEFAULT_OLLAMA_MODEL)));
            // Opt-in, disabled by default (docs/09 §5.1) — no Settings UI yet
            // to turn it on.
            let search_engine: Arc<dyn SearchEngine> = Arc::new(ContexaSearchEngine::default());

            let pipeline = Arc::new(PipelineManager::new(
                Arc::clone(&context) as Arc<dyn ContextEngine>,
                Arc::clone(&memory_engine),
                Arc::clone(&vision) as Arc<dyn VisionEngine>,
                search_engine,
                prompt_builder,
                provider,
                PipelineConfig::default(),
            ));
            let orchestrator = Arc::new(ContexaOrchestrator::new(
                Arc::clone(&context) as Arc<dyn ContextEngine>,
                pipeline,
            ));

            app.manage(vision);
            app.manage(context);
            app.manage(orchestrator);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
