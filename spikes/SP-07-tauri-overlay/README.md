## SP-07 — Tauri overlay + global hotkey

**Spec:** `docs/22_Technical_Spike_Plan.md` (SP-07), `docs/12_UI_UX.md`

### Goal

Validate: overlay opens within 200ms of `Alt+Space` with a **preloaded** WebView.

### App location

`spikes/SP-07-tauri-overlay/sp07-app/` (pnpm + Tauri 2 + React TS)

### How to run

```powershell
Set-Location D:\Contexa\spikes\SP-07-tauri-overlay\sp07-app
pnpm tauri dev
```

1. Window starts **hidden** (preloaded).
2. Press **`Alt+Space`** to show/hide overlay.
3. Click **Run open-latency bench (100×)** → reads p50/p95 open + focus timings.
4. Copy output into `../report.md`.

### Notes / gotchas

- On Windows, `Alt+Space` is also the system menu shortcut for the focused window — if toggle feels flaky, try focusing another app then pressing the hotkey; note result in the report.
- Transparency/decorations/alwaysOnTop come from `src-tauri/tauri.conf.json`.
- Benchmark measures **show→focus** on an already-created window (preload path), not cold process start.

### Pass criteria (`docs/22`)

| Metric | Target |
|--------|--------|
| Open latency p50 | < 150 ms |
| Open latency p95 | < 200 ms |
| Focus steal duration | < 100 ms |
