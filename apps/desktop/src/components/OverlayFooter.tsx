import type { ReactNode } from "react";
import { Clock, Gear } from "@phosphor-icons/react";

// docs/12 §5.1 footer row. No Close button here — the native title bar
// (docs/12 §5.3 pivot: regular resizable window) already has one; a second
// close action with the same intent would be a duplicate control. Esc still
// hides the window (App.tsx).
export function OverlayFooter() {
  return (
    <div className="flex items-center gap-1 border-t border-border px-4 py-2">
      <FooterButton icon={<Clock size={14} />} label="Timeline" disabled title="Timeline — coming soon" />
      <FooterButton icon={<Gear size={14} />} label="Settings" disabled title="Settings — coming soon" />
    </div>
  );
}

function FooterButton({
  icon,
  label,
  disabled,
  title,
}: {
  icon: ReactNode;
  label: string;
  disabled?: boolean;
  title: string;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      title={title}
      className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-text-secondary transition-colors
        hover:bg-bg-secondary hover:text-text-primary
        focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent
        disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
    >
      {icon}
      {label}
    </button>
  );
}
