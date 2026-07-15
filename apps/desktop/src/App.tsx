import "./App.css";

function App() {
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

      <section className="context">No context yet — engines land in Phase 1.</section>

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
