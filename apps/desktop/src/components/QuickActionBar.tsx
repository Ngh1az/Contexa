import type { RequestActionKind } from "../lib/tauri";

// docs/12 §6 — flat text, no icon-bubble buttons (design direction: minimal,
// Claude.ai/Codex-CLI style, not a colored toolbar).
const ACTIONS: { action: RequestActionKind; label: string; shortcut: string; enabled: boolean }[] = [
  { action: "explain", label: "Explain", shortcut: "E", enabled: true },
  { action: "summarize", label: "Summarize", shortcut: "S", enabled: true },
  // Translate needs a language picker (docs/12 §11 has no spec for it yet) —
  // shown but disabled rather than silently wired to a hardcoded language.
  { action: "translate", label: "Translate", shortcut: "T", enabled: false },
  { action: "search", label: "Search", shortcut: "/", enabled: true },
];

export function QuickActionBar({
  onAction,
  disabled,
}: {
  onAction: (action: RequestActionKind) => void;
  disabled: boolean;
}) {
  return (
    <div className="flex items-center gap-1 px-4 py-1.5">
      {ACTIONS.map(({ action, label, shortcut, enabled }) => (
        <button
          key={action}
          type="button"
          disabled={disabled || !enabled}
          onClick={() => onAction(action)}
          title={enabled ? `${label} (${shortcut})` : `${label} — coming soon`}
          className="rounded-md px-2 py-1 text-xs text-text-secondary transition-colors
            hover:bg-bg-secondary hover:text-text-primary
            focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent
            disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
        >
          {label}
        </button>
      ))}
    </div>
  );
}
