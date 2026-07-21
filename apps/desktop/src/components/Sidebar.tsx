import { Clock, Gear, Plus } from "@phosphor-icons/react";

// docs/12 §5.1/§16.1 rail — icon-only nav, no chat-history list (spec §2).
// Timeline stays disabled (unimplemented, spec §7); Settings opens the
// minimal panel from SettingsPanel.
export function Sidebar({
  onNewConversation,
  onOpenSettings,
  settingsActive,
  disabled,
}: {
  onNewConversation: () => void;
  onOpenSettings: () => void;
  settingsActive: boolean;
  disabled: boolean;
}) {
  return (
    <div className="flex w-12 shrink-0 flex-col items-center gap-3 border-r border-border bg-bg-secondary pt-3">
      <button
        type="button"
        onClick={onNewConversation}
        disabled={disabled}
        title="New conversation"
        className="flex h-7 w-7 items-center justify-center rounded-lg bg-accent text-bg-primary transition-colors
          hover:bg-accent-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent
          disabled:cursor-not-allowed disabled:opacity-40"
      >
        <Plus size={16} weight="bold" />
      </button>
      <button
        type="button"
        disabled
        title="Timeline — coming soon"
        className="flex h-7 w-7 items-center justify-center rounded-lg text-text-secondary
          disabled:cursor-not-allowed disabled:opacity-40"
      >
        <Clock size={16} />
      </button>
      <button
        type="button"
        onClick={onOpenSettings}
        title="Settings"
        aria-pressed={settingsActive}
        className={`flex h-7 w-7 items-center justify-center rounded-lg transition-colors
          hover:bg-bg-primary hover:text-text-primary
          focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent
          ${settingsActive ? "bg-bg-primary text-text-primary" : "text-text-secondary"}`}
      >
        <Gear size={16} />
      </button>
    </div>
  );
}
