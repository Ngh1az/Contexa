import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { motion } from "motion/react";
import { ContextIndicator } from "./components/ContextIndicator";
import { QuickActionBar, ACTION_LABELS } from "./components/QuickActionBar";
import { MessageList } from "./components/MessageList";
import { Sidebar } from "./components/Sidebar";
import { SettingsPanel } from "./components/SettingsPanel";
import { initialOverlayState, overlayReducer } from "./lib/overlayState";
import { useTheme } from "./lib/theme";
import {
  type ContextSnapshot,
  type RequestActionKind,
  cancelRequest,
  getCurrentContext,
  handleRequest,
  hideOverlay,
  onAiChunk,
  onAiComplete,
  onAiError,
  onOverlayFocus,
} from "./lib/tauri";

const CONTEXT_POLL_MS = 1500;

function useCurrentContext() {
  const [context, setContext] = useState<ContextSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const poll = () => {
      getCurrentContext()
        .then((result) => {
          if (!cancelled) {
            setContext(result);
            setError(null);
          }
        })
        .catch((err: unknown) => {
          if (!cancelled) setError(String(err));
        });
    };

    poll();
    const id = setInterval(poll, CONTEXT_POLL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return { context, error };
}

function App() {
  const [state, dispatch] = useReducer(overlayReducer, initialOverlayState);
  const { context, error: contextError } = useCurrentContext();
  const { preference, setPreference } = useTheme();
  const [view, setView] = useState<"chat" | "settings">("chat");
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // ai-chunk/ai-complete/ai-error (docs/12 §7.2) — wired once for the life
  // of this preloaded window.
  useEffect(() => {
    const unlistenChunk = onAiChunk((e) =>
      dispatch({ type: "chunk", requestId: e.request_id, content: e.content }),
    );
    const unlistenComplete = onAiComplete((e) => dispatch({ type: "complete", requestId: e.request_id }));
    const unlistenError = onAiError((e) =>
      dispatch({ type: "error", requestId: e.request_id, message: e.error }),
    );
    return () => {
      unlistenChunk.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Reset to a clean slate + focus input every time the overlay reopens
  // (docs/12 §5.2: Hidden -> Input) — the window is preloaded, not remounted.
  useEffect(() => {
    inputRef.current?.focus();
    const unlisten = onOverlayFocus(() => {
      dispatch({ type: "reset" });
      setQuery("");
      setView("chat");
      inputRef.current?.focus();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const submit = useCallback(async (action: RequestActionKind, actionQuery?: string) => {
    const res = await handleRequest({ action, query: actionQuery, stream: true });
    if (res.status === "rejected") {
      dispatch({ type: "rejected", reason: res.reason ?? "Request rejected" });
      return;
    }
    dispatch({ type: "submit", requestId: res.request_id, query: actionQuery ?? ACTION_LABELS[action] ?? action });
  }, []);

  const onSubmitQuery = () => {
    const trimmed = query.trim();
    if (!trimmed || state.phase === "processing") return;
    setQuery("");
    void submit("chat", trimmed);
  };

  const onEscape = useCallback(async () => {
    if (state.phase === "processing" && state.requestId) {
      await cancelRequest(state.requestId).catch(() => undefined);
      dispatch({ type: "cancel" });
    }
    await hideOverlay();
  }, [state.phase, state.requestId]);

  const onNewConversation = useCallback(async () => {
    if (state.phase === "processing" && state.requestId) {
      await cancelRequest(state.requestId).catch(() => undefined);
    }
    dispatch({ type: "reset" });
    setQuery("");
    setView("chat");
    inputRef.current?.focus();
  }, [state.phase, state.requestId]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.15, ease: "easeOut" }}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          void onEscape();
        }
      }}
      className="flex h-full w-full overflow-hidden bg-bg-primary text-text-primary"
    >
      <Sidebar
        onNewConversation={() => void onNewConversation()}
        onOpenSettings={() => setView((v) => (v === "settings" ? "chat" : "settings"))}
        settingsActive={view === "settings"}
        disabled={state.phase === "processing"}
      />

      {view === "settings" ? (
        <SettingsPanel preference={preference} onChange={setPreference} onClose={() => setView("chat")} />
      ) : (
        <div className="flex flex-1 flex-col overflow-hidden">
          <div className="border-b border-border px-4 py-1.5">
            <ContextIndicator context={context} error={contextError} />
          </div>

          <MessageList phase={state.phase} messages={state.messages} error={state.error} />

          <QuickActionBar disabled={state.phase === "processing"} onAction={(action) => void submit(action)} />

          <div className="border-t border-border px-4 py-3">
            <textarea
              ref={inputRef}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSubmitQuery();
                }
              }}
              placeholder="Ask anything about your screen…"
              rows={1}
              className="w-full resize-none rounded-lg border border-border bg-bg-secondary px-3 py-2 text-sm text-text-primary
                placeholder:text-text-secondary focus:outline-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
            />
          </div>
        </div>
      )}
    </motion.div>
  );
}

export default App;
