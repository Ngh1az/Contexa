import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Mirrors contexa_core::ContextSnapshot (crates/contexa-core/src/types.rs) —
// flat shape, matches the DB row directly; see that module's doc comment.
interface ContextSnapshot {
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

const POLL_INTERVAL_MS = 1500;

function useCurrentContext() {
  const [context, setContext] = useState<ContextSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const poll = () => {
      invoke<ContextSnapshot | null>("get_current_context")
        .then((result) => {
          if (!cancelled) {
            setContext(result);
            setError(null);
          }
        })
        .catch((err: unknown) => {
          if (!cancelled) {
            setError(String(err));
          }
        });
    };

    poll();
    const id = setInterval(poll, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return { context, error };
}

function ContextPanel({
  context,
  error,
}: {
  context: ContextSnapshot | null;
  error: string | null;
}) {
  if (error) {
    return <span className="context-empty">Context unavailable: {error}</span>;
  }
  if (!context) {
    return <span className="context-empty">No context yet — switch windows to capture one.</span>;
  }

  const detail = context.url ?? context.document_path;

  return (
    <div className="context-row">
      <span className="context-app">{context.process_name}</span>
      <span className="context-title" title={context.window_title}>
        {context.window_title}
      </span>
      {detail && (
        <span className="context-detail" title={detail}>
          {detail}
        </span>
      )}
    </div>
  );
}

function App() {
  const { context, error } = useCurrentContext();

  return (
    <main className="overlay">
      <header className="bar">
        <span className="prompt">Ask anything about your screen…</span>
      </header>

      <section className="actions">
        <button type="button" disabled>
          Explain
        </button>
        <button type="button" disabled>
          Summarize
        </button>
        <button type="button" disabled>
          Translate
        </button>
        <button type="button" disabled>
          Search
        </button>
      </section>

      <section className="context">
        <ContextPanel context={context} error={error} />
      </section>

      <section className="body">
        <p>
          Hotkey: <kbd>Alt</kbd>+<kbd>Space</kbd> (toggle). Window is preloaded at startup
          (hidden → show).
        </p>
      </section>
    </main>
  );
}

export default App;
