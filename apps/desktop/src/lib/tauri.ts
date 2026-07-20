// Thin, typed wrapper around the Tauri IPC surface docs/03_API_Interface_Specification.md
// §5.2 defines and apps/desktop/src-tauri/src/lib.rs implements. Kept
// separate from components so the overlay UI never calls `invoke`/`listen`
// directly with untyped strings.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// `pnpm dev` can be opened in a plain browser tab for quick UI iteration —
// `@tauri-apps/api` throws synchronously outside a real Tauri webview
// (no `__TAURI_INTERNALS__`). Guarded once here so every export below is
// safe to call in either context; production (real Tauri) behavior is
// unchanged since `isTauri` is always true there.
const isTauri = "__TAURI_INTERNALS__" in window;
const noopUnlisten: UnlistenFn = () => {};

// Mirrors contexa_core::ContextSnapshot (crates/contexa-core/src/types.rs).
export interface ContextSnapshot {
  id: string;
  timestamp: string;
  window_title: string;
  process_name: string;
  process_id: number;
  hwnd: number | null;
  url: string | null;
  document_path: string | null;
  visible_text: string | null;
  selected_text: string | null;
  metadata: Record<string, string>;
  language: string | null;
  capture_method: "uia" | "ocr" | "hybrid";
}

export type RequestActionKind = "chat" | "explain" | "summarize" | "translate" | "search" | "recall";

export interface HandleRequestParams {
  action: RequestActionKind;
  query?: string;
  target_lang?: string;
  stream?: boolean;
}

export interface HandleRequestResponse {
  request_id: string;
  status: "accepted" | "rejected";
  reason: string | null;
}

export interface AiChunkEvent {
  request_id: string;
  content: string;
  done: boolean;
}

export interface AiCompleteEvent {
  request_id: string;
  total_tokens: number;
  latency_ms: number;
}

export interface AiErrorEvent {
  request_id: string;
  error: string;
  code: string;
}

export function getCurrentContext(): Promise<ContextSnapshot | null> {
  if (!isTauri) return Promise.resolve(null);
  return invoke("get_current_context");
}

export function handleRequest(params: HandleRequestParams): Promise<HandleRequestResponse> {
  if (!isTauri) {
    return Promise.resolve({ request_id: "", status: "rejected", reason: "Tauri runtime unavailable" });
  }
  return invoke("handle_request", { params });
}

export function cancelRequest(requestId: string): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return invoke("cancel_request", { requestId });
}

export function onAiChunk(handler: (event: AiChunkEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(noopUnlisten);
  return listen<AiChunkEvent>("ai-chunk", (e) => handler(e.payload));
}

export function onAiComplete(handler: (event: AiCompleteEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(noopUnlisten);
  return listen<AiCompleteEvent>("ai-complete", (e) => handler(e.payload));
}

export function onAiError(handler: (event: AiErrorEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(noopUnlisten);
  return listen<AiErrorEvent>("ai-error", (e) => handler(e.payload));
}

/** Hides the overlay window (Escape / close button) — docs/12 §5.2. */
export function hideOverlay(): Promise<void> {
  if (!isTauri) return Promise.resolve();
  return getCurrentWindow().hide();
}

/** Fires each time the preloaded overlay window regains focus (Alt+Space reopen). */
export function onOverlayFocus(handler: () => void): Promise<UnlistenFn> {
  if (!isTauri) return Promise.resolve(noopUnlisten);
  return getCurrentWindow().listen("tauri://focus", () => handler());
}
