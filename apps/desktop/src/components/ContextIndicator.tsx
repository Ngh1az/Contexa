import { AppWindow } from "@phosphor-icons/react";
import type { ContextSnapshot } from "../lib/tauri";

// docs/12 §5.4 — single line, one glance. App-specific icon variety in the
// spec's mockup is cosmetic notation, not a requirement; one neutral icon
// keeps this flat rather than building an app→icon lookup table (YAGNI).
export function ContextIndicator({
  context,
  error,
}: {
  context: ContextSnapshot | null;
  error: string | null;
}) {
  if (error) {
    return <p className="text-xs text-text-secondary">Context unavailable: {error}</p>;
  }
  if (!context) {
    return <p className="text-xs text-text-secondary">No active window — switch focus to capture context.</p>;
  }

  const detail = context.url ?? context.document_path;

  return (
    <div className="flex items-center gap-1.5 text-xs text-text-secondary">
      <AppWindow size={14} weight="regular" className="shrink-0" />
      <span className="font-medium text-text-primary">{context.process_name}</span>
      <span className="truncate" title={context.window_title}>
        {context.window_title}
      </span>
      {detail && (
        <span className="truncate text-text-secondary/70" title={detail}>
          {detail}
        </span>
      )}
    </div>
  );
}
