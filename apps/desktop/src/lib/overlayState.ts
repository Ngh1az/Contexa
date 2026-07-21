// Overlay interaction state machine — docs/12_UI_UX.md §5.2. Pure so it's
// unit-testable without mounting React or a Tauri webview.

export type OverlayPhase = "idle" | "processing" | "streaming";

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
}

export interface OverlayState {
  phase: OverlayPhase;
  requestId: string | null;
  messages: Message[];
  error: string | null;
}

export const initialOverlayState: OverlayState = {
  phase: "idle",
  requestId: null,
  messages: [],
  error: null,
};

export type OverlayAction =
  | { type: "reset" }
  | { type: "submit"; requestId: string; query: string }
  | { type: "rejected"; reason: string }
  | { type: "chunk"; requestId: string; content: string }
  | { type: "complete"; requestId: string }
  | { type: "error"; requestId: string; message: string }
  | { type: "cancel" };

export function overlayReducer(state: OverlayState, action: OverlayAction): OverlayState {
  switch (action.type) {
    case "reset":
      return initialOverlayState;
    case "submit":
      return {
        phase: "processing",
        requestId: action.requestId,
        error: null,
        messages: [
          ...state.messages,
          { id: `${action.requestId}-user`, role: "user", content: action.query },
          { id: action.requestId, role: "assistant", content: "" },
        ],
      };
    case "rejected":
      return { ...state, phase: "idle", error: action.reason };
    case "chunk":
      // ai-chunk events are broadcast per-window, not scoped to a request —
      // ignore anything not addressed to the in-flight request (stale/cancelled).
      if (action.requestId !== state.requestId) return state;
      return {
        ...state,
        phase: "streaming",
        messages: state.messages.map((m) =>
          m.id === action.requestId ? { ...m, content: m.content + action.content } : m,
        ),
      };
    case "complete":
      if (action.requestId !== state.requestId) return state;
      return { ...state, phase: "idle" };
    case "error":
      if (action.requestId !== state.requestId) return state;
      return { ...state, phase: "idle", error: action.message };
    case "cancel":
      return { ...state, phase: "idle", requestId: null };
    default:
      return state;
  }
}
