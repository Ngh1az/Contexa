import type { ThemePreference } from "../lib/theme";

// docs/12 §9 — minimal slice: only General → Theme ships here (spec §3.4,
// §7). Other sections (AI Provider, Capture, Memory, Search, Privacy, MCP,
// About) are out of scope and intentionally absent, not stubbed.
const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function SettingsPanel({
  preference,
  onChange,
  onClose,
}: {
  preference: ThemePreference;
  onChange: (value: ThemePreference) => void;
  onClose: () => void;
}) {
  return (
    <div className="flex flex-1 flex-col overflow-y-auto px-4 py-3">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-text-primary">Settings</h2>
        <button
          type="button"
          onClick={onClose}
          className="text-xs text-text-secondary hover:text-text-primary
            focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
        >
          Back
        </button>
      </div>
      <section>
        <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-text-secondary">General</h3>
        <label className="flex items-center justify-between text-sm text-text-primary">
          Theme
          <select
            value={preference}
            onChange={(e) => onChange(e.target.value as ThemePreference)}
            className="rounded-md border border-border bg-bg-secondary px-2 py-1 text-sm text-text-primary
              focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
          >
            {THEME_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>
      </section>
    </div>
  );
}
