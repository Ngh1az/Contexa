import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [benchOut, setBenchOut] = useState("Ready. Press Alt+Space to toggle overlay.");
  const [busy, setBusy] = useState(false);

  async function runBench() {
    setBusy(true);
    setBenchOut("Running 100 show/hide iterations…");
    try {
      const out = await invoke<string>("run_open_latency_bench", { iterations: 100 });
      setBenchOut(out);
    } catch (e) {
      setBenchOut(String(e));
    } finally {
      setBusy(false);
    }
  }

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

      <section className="context">📄 VS Code — SP-07 overlay spike</section>

      <section className="body">
        <p>
          Hotkey: <kbd>Alt</kbd>+<kbd>Space</kbd> (toggle). Window is preloaded at startup
          (hidden → show).
        </p>
        <button type="button" className="primary" disabled={busy} onClick={runBench}>
          {busy ? "Benchmarking…" : "Run open-latency bench (100×)"}
        </button>
        <pre className="bench">{benchOut}</pre>
      </section>
    </main>
  );
}

export default App;
