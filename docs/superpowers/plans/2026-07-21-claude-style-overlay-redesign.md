# Claude-style Overlay Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle and restructure the Contexa desktop overlay (`apps/desktop/src`) to match Claude.ai's visual language — warm cream/dark palette, left icon-rail, multi-turn chat with user bubbles, bottom-pinned input, and a light/dark theme toggle.

**Architecture:** Pure-logic-first: extend the existing `overlayState.ts` reducer from a single `response: string` to a `messages: Message[]` array (same pattern already used there — pure, unit-testable without mounting React), and add a parallel pure `theme.ts` module for light/dark resolution. Presentational components (`Sidebar`, `MessageList`, `SettingsPanel`) consume that state; `App.tsx` wires them into a `flex-row` layout (rail + main column) replacing the current `flex-col` single-response layout.

**Tech Stack:** React 19, TypeScript, TailwindCSS v4 (CSS-first `@theme`, no typography plugin installed), Vitest 4 (`environment: "node"`, no jsdom/RTL — component changes are verified manually via the dev server, consistent with the project's current test coverage which only covers pure logic).

**Spec:** `docs/superpowers/specs/2026-07-21-claude-style-overlay-redesign-design.md`

## Global Constraints

- Scope is `apps/desktop/src` only — no Rust/Tauri backend changes (spec §"Scope").
- No new npm dependencies. No Tauri store plugin (none installed); theme persists via `localStorage` (spec §4.2).
- Sidebar is icon-only nav (New / Timeline / Settings) — no chat-history list (spec §2).
- Settings panel ships with exactly one control (General → Theme). No other settings sections (spec §3.4, §7).
- Conversation resets when the overlay regains focus (`tauri://focus`) — same trigger as today, not persisted (spec §2, §4.1).
- Color tokens: dark `bg #262624 / accent #d97757`, light `bg #faf9f5 / accent #c15f3c` (spec §3.2 full table) — hue direction is locked, exact hex may shift only if a contrast check in Task 9 fails AA 4.5:1 for a text usage.
- Timeline stays a disabled stub — not implemented in this plan (spec §7).

---

## Task 1: Theme color tokens (light + dark)

**Files:**
- Modify: `apps/desktop/src/index.css`

**Interfaces:**
- Produces: CSS custom properties `--color-bg-primary`, `--color-bg-secondary`, `--color-bg-tertiary` (new), `--color-text-primary`, `--color-text-secondary`, `--color-accent`, `--color-accent-hover`, `--color-success`, `--color-warning`, `--color-error`, `--color-border` — each overridden under `:root[data-theme="light"]`. Tailwind v4 auto-generates utilities from these (`bg-bg-tertiary`, etc.), same mechanism the existing 10 tokens already use.

- [ ] **Step 1: Replace the `@theme` block and add the light override block**

In `apps/desktop/src/index.css`, replace the existing `@theme { ... }` block (lines 15-29) with:

```css
/* docs/12 §12.1 — Dark Theme design tokens (default) */
@theme {
  --color-bg-primary: #262624;
  --color-bg-secondary: #1e1e1c;
  --color-bg-tertiary: #3a3a37;
  --color-text-primary: #f0eee6;
  --color-text-secondary: #8f8d86;
  --color-accent: #d97757;
  --color-accent-hover: #e08966;
  --color-success: #00b894;
  --color-warning: #fdcb6e;
  --color-error: #e17055;
  --color-border: #3a3a37;

  --font-sans: "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, monospace;
}

/* docs/12 §12.1 — Light Theme overrides, applied via [data-theme="light"]
   on <html> (src/lib/theme.ts sets this attribute). */
:root[data-theme="light"] {
  --color-bg-primary: #faf9f5;
  --color-bg-secondary: #f4f3ee;
  --color-bg-tertiary: #e8e6dc;
  --color-text-primary: #1f1e1d;
  --color-text-secondary: #6b6a66;
  --color-accent: #c15f3c;
  --color-accent-hover: #ad5434;
  --color-success: #00966f;
  --color-warning: #b8860b;
  --color-error: #c44f36;
  --color-border: #e5e3da;
}
```

- [ ] **Step 2: Typecheck and build to confirm the CSS still compiles**

Run: `pnpm -C apps/desktop run typecheck && pnpm -C apps/desktop run build`
Expected: both succeed with no errors (Tailwind v4 has no separate lint step for `@theme`; a build failure would show as a PostCSS/Vite error).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/index.css
git commit -m "feat(desktop): add Claude-style light/dark color tokens"
```

---

## Task 2: Theme resolution logic + hook

**Files:**
- Create: `apps/desktop/src/lib/theme.ts`
- Test: `apps/desktop/src/lib/theme.test.ts`

**Interfaces:**
- Produces: `ThemePreference = "system" | "light" | "dark"`, `ResolvedTheme = "light" | "dark"`, pure functions `resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme` and `readStoredPreference(getItem: (key: string) => string | null): ThemePreference`, and a React hook `useTheme(): { preference: ThemePreference; resolved: ResolvedTheme; setPreference: (next: ThemePreference) => void }`.
- Consumes: nothing from other tasks.

- [ ] **Step 1: Write the failing tests for the pure functions**

Create `apps/desktop/src/lib/theme.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { readStoredPreference, resolveTheme } from "./theme";

describe("resolveTheme", () => {
  it("returns dark when preference is system and OS prefers dark", () => {
    expect(resolveTheme("system", true)).toBe("dark");
  });

  it("returns light when preference is system and OS prefers light", () => {
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("returns the explicit preference regardless of OS setting", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

describe("readStoredPreference", () => {
  it("returns the stored value when it is a valid explicit preference", () => {
    expect(readStoredPreference(() => "light")).toBe("light");
    expect(readStoredPreference(() => "dark")).toBe("dark");
  });

  it("falls back to system when nothing is stored", () => {
    expect(readStoredPreference(() => null)).toBe("system");
  });

  it("falls back to system when the stored value is not a valid theme", () => {
    expect(readStoredPreference(() => "purple")).toBe("system");
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm -C apps/desktop test -- src/lib/theme.test.ts`
Expected: FAIL — `Cannot find module './theme'` (file doesn't exist yet).

- [ ] **Step 3: Implement `theme.ts`**

Create `apps/desktop/src/lib/theme.ts`:

```ts
// Pure resolution logic kept separate from the DOM-touching hook so it's
// unit-testable without mounting React or jsdom (vitest.config.ts runs
// environment: "node" — same separation overlayState.ts uses for the
// overlay's own reducer).
import { useCallback, useEffect, useState } from "react";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "contexa-theme";

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  if (preference === "system") return systemPrefersDark ? "dark" : "light";
  return preference;
}

export function readStoredPreference(getItem: (key: string) => string | null): ThemePreference {
  const stored = getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" ? stored : "system";
}

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readStoredPreference((key) => window.localStorage.getItem(key)),
  );
  const [resolved, setResolved] = useState<ResolvedTheme>(() => resolveTheme(preference, systemPrefersDark()));

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", resolved);
  }, [resolved]);

  useEffect(() => {
    if (preference !== "system") {
      setResolved(resolveTheme(preference, false));
      return;
    }
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setResolved(resolveTheme("system", mql.matches));
    update();
    mql.addEventListener("change", update);
    return () => mql.removeEventListener("change", update);
  }, [preference]);

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    window.localStorage.setItem(STORAGE_KEY, next);
  }, []);

  return { preference, resolved, setPreference };
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm -C apps/desktop test -- src/lib/theme.test.ts`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/lib/theme.ts apps/desktop/src/lib/theme.test.ts
git commit -m "feat(desktop): add theme preference resolution + useTheme hook"
```

---

## Task 3: Multi-turn message state (`overlayState.ts`)

**Files:**
- Modify: `apps/desktop/src/lib/overlayState.ts`
- Modify: `apps/desktop/src/lib/overlayState.test.ts` (full rewrite)

**Interfaces:**
- Produces: `Message { id: string; role: "user" | "assistant"; content: string }`, `OverlayState { phase: OverlayPhase; requestId: string | null; messages: Message[]; error: string | null }`, `OverlayAction` with `submit` now carrying `{ requestId: string; query: string }`.
- Consumes: nothing from other tasks (pure module, same as before).

- [ ] **Step 1: Write the failing tests (full replacement)**

Replace all of `apps/desktop/src/lib/overlayState.test.ts` with:

```ts
import { describe, expect, it } from "vitest";
import { initialOverlayState, overlayReducer } from "./overlayState";

describe("overlayReducer", () => {
  it("submit appends a user message and an empty assistant placeholder, moves to processing", () => {
    const next = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hello" });
    expect(next.phase).toBe("processing");
    expect(next.requestId).toBe("r1");
    expect(next.error).toBeNull();
    expect(next.messages).toEqual([
      { id: "r1-user", role: "user", content: "hello" },
      { id: "r1", role: "assistant", content: "" },
    ]);
  });

  it("a second submit appends onto existing messages (multi-turn)", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "first" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    state = overlayReducer(state, { type: "submit", requestId: "r2", query: "second" });
    expect(state.messages).toHaveLength(4);
    expect(state.messages[2]).toEqual({ id: "r2-user", role: "user", content: "second" });
  });

  it("first chunk moves processing to streaming and appends content to the assistant message", () => {
    const processing = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    const next = overlayReducer(processing, { type: "chunk", requestId: "r1", content: "Hel" });
    expect(next.phase).toBe("streaming");
    expect(next.messages[1].content).toBe("Hel");
  });

  it("chunks accumulate in order", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "Hel" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "lo" });
    expect(state.messages[1].content).toBe("Hello");
  });

  it("chunk from a stale request id is ignored", () => {
    const state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    const next = overlayReducer(state, { type: "chunk", requestId: "stale", content: "x" });
    expect(next).toBe(state);
  });

  it("complete returns to idle while keeping the accumulated message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    expect(state.phase).toBe("idle");
    expect(state.messages[1].content).toBe("answer");
  });

  it("error from the in-flight request surfaces as a banner and returns to idle, keeping partial message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "partial" });
    const next = overlayReducer(state, { type: "error", requestId: "r1", message: "provider down" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("provider down");
    expect(next.messages[1].content).toBe("partial");
  });

  it("rejected (synchronous handle_request failure) surfaces a banner without touching existing messages", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "first" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    const messagesBefore = state.messages;
    const next = overlayReducer(state, { type: "rejected", reason: "unknown action" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("unknown action");
    expect(next.messages).toBe(messagesBefore);
  });

  it("cancel returns to idle and clears the request id but keeps partial message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "partial" });
    state = overlayReducer(state, { type: "cancel" });
    expect(state.phase).toBe("idle");
    expect(state.requestId).toBeNull();
    expect(state.messages[1].content).toBe("partial");
  });

  it("reset returns to the initial state (overlay reopened)", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    expect(overlayReducer(state, { type: "reset" })).toEqual(initialOverlayState);
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm -C apps/desktop test -- src/lib/overlayState.test.ts`
Expected: FAIL — type errors / assertion failures against the current single-`response` shape (e.g. `next.messages` is undefined).

- [ ] **Step 3: Rewrite `overlayState.ts`**

Replace all of `apps/desktop/src/lib/overlayState.ts` with:

```ts
// Overlay interaction state machine — docs/12_UI_UX.md §5.2. Pure so it's
// unit-testable without mounting React or a Tauri webview.

export type OverlayPhase = "idle" | "processing" | "streaming";

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
}

export interface OverlayState {
  phase: OverlayPhase;
  requestId: string | null;
  messages: Message[];
  error: string | null;
}

export const initialOverlayState: OverlayState = {
  phase: "idle",
  requestId: null,
  messages: [],
  error: null,
};

export type OverlayAction =
  | { type: "reset" }
  | { type: "submit"; requestId: string; query: string }
  | { type: "rejected"; reason: string }
  | { type: "chunk"; requestId: string; content: string }
  | { type: "complete"; requestId: string }
  | { type: "error"; requestId: string; message: string }
  | { type: "cancel" };

export function overlayReducer(state: OverlayState, action: OverlayAction): OverlayState {
  switch (action.type) {
    case "reset":
      return initialOverlayState;
    case "submit":
      return {
        phase: "processing",
        requestId: action.requestId,
        error: null,
        messages: [
          ...state.messages,
          { id: `${action.requestId}-user`, role: "user", content: action.query },
          { id: action.requestId, role: "assistant", content: "" },
        ],
      };
    case "rejected":
      return { ...state, phase: "idle", error: action.reason };
    case "chunk":
      // ai-chunk events are broadcast per-window, not scoped to a request —
      // ignore anything not addressed to the in-flight request (stale/cancelled).
      if (action.requestId !== state.requestId) return state;
      return {
        ...state,
        phase: "streaming",
        messages: state.messages.map((m) =>
          m.id === action.requestId ? { ...m, content: m.content + action.content } : m,
        ),
      };
    case "complete":
      if (action.requestId !== state.requestId) return state;
      return { ...state, phase: "idle" };
    case "error":
      if (action.requestId !== state.requestId) return state;
      return { ...state, phase: "idle", error: action.message };
    case "cancel":
      return { ...state, phase: "idle", requestId: null };
    default:
      return state;
  }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm -C apps/desktop test -- src/lib/overlayState.test.ts`
Expected: PASS (10 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/lib/overlayState.ts apps/desktop/src/lib/overlayState.test.ts
git commit -m "feat(desktop): switch overlay state from single response to message list"
```

---

## Task 4: Quick action labels export

**Files:**
- Modify: `apps/desktop/src/components/QuickActionBar.tsx`

**Interfaces:**
- Produces: `ACTION_LABELS: Partial<Record<RequestActionKind, string>>` — derived from the existing `ACTIONS` array (single source of truth for labels).
- Consumes: nothing new.

This is a trivial one-line derivation (ponytail: no dedicated test needed) — it's consumed by Task 8 to give quick-action-triggered messages (Explain/Summarize/Search, which have no typed query) a readable user-bubble label instead of the raw action id.

- [ ] **Step 1: Add the export**

In `apps/desktop/src/components/QuickActionBar.tsx`, after the `ACTIONS` array (after line 12), add:

```ts
// Reused by App.tsx to label the user-bubble for quick actions, which have
// no typed query — single source of truth, no separate label list to drift.
export const ACTION_LABELS: Partial<Record<RequestActionKind, string>> = Object.fromEntries(
  ACTIONS.map(({ action, label }) => [action, label]),
);
```

- [ ] **Step 2: Typecheck**

Run: `pnpm -C apps/desktop run typecheck`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/QuickActionBar.tsx
git commit -m "feat(desktop): export quick action labels for user-bubble text"
```

---

## Task 5: `MessageList` component (replaces `ResponsePanel`)

**Files:**
- Create: `apps/desktop/src/components/MessageList.tsx`
- Delete: `apps/desktop/src/components/ResponsePanel.tsx`

**Interfaces:**
- Consumes: `Message`, `OverlayPhase` from `../lib/overlayState` (Task 3).
- Produces: `MessageList({ phase: OverlayPhase; messages: Message[]; error: string | null })` — a `flex-1` scrollable region, mounted by `App.tsx` (Task 8) between the context indicator and the quick-action bar.

No dedicated unit test — this repo has no component-test infra (`vitest.config.ts` runs `environment: "node"`, no jsdom/RTL installed; see plan Tech Stack). Verified manually in Task 9.

- [ ] **Step 1: Create the component**

Create `apps/desktop/src/components/MessageList.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Check, Copy } from "@phosphor-icons/react";
import type { Message, OverlayPhase } from "../lib/overlayState";

// docs/12 §7.3 (markdown/code rendering) + Claude.ai-style bubbles: user
// messages get a bubble (bg-tertiary), assistant responses stay plain text
// (no bubble) — same minimal direction ResponsePanel used, extended to a
// scrollable multi-turn list.
export function MessageList({
  phase,
  messages,
  error,
}: {
  phase: OverlayPhase;
  messages: Message[];
  error: string | null;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll) scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages, autoScroll]);

  const onScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    setAutoScroll(atBottom);
  };

  if (messages.length === 0 && !error) {
    return <div className="flex-1" />;
  }

  const lastMessage = messages[messages.length - 1];

  return (
    <div ref={scrollRef} onScroll={onScroll} className="flex-1 overflow-y-auto px-4 py-3">
      {messages.map((message) => (
        <MessageBubble key={message.id} message={message} />
      ))}
      {phase === "processing" && lastMessage?.content.length === 0 && (
        <div className="flex flex-col gap-2 py-1" aria-label="Loading response">
          <div className="h-3 w-3/4 animate-pulse rounded bg-bg-secondary" />
          <div className="h-3 w-1/2 animate-pulse rounded bg-bg-secondary" />
        </div>
      )}
      {error && <p className="pt-2 text-sm text-error">{error}</p>}
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  if (message.role === "user") {
    return (
      <div className="mb-3 flex justify-end">
        <div className="max-w-[75%] rounded-[12px_12px_2px_12px] bg-bg-tertiary px-3 py-1.5 text-sm text-text-primary">
          {message.content}
        </div>
      </div>
    );
  }

  if (message.content.length === 0) return null;

  return (
    <div className="group relative mb-3">
      <CopyButton text={message.content} />
      <div
        className="prose-invert max-w-none text-sm leading-relaxed text-text-primary
          [&_code]:font-mono [&_code]:text-[13px] [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-bg-secondary [&_pre]:p-3
          [&_a]:text-accent [&_a:hover]:text-accent-hover"
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
          {message.content}
        </ReactMarkdown>
      </div>
    </div>
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <button
      type="button"
      onClick={copy}
      title="Copy response"
      className="absolute right-2 top-2 rounded-md p-1.5 text-text-secondary opacity-0 transition-opacity
        hover:text-text-primary focus-visible:opacity-100 focus-visible:outline focus-visible:outline-2
        focus-visible:outline-accent group-hover:opacity-100"
    >
      {copied ? <Check size={14} weight="bold" /> : <Copy size={14} />}
    </button>
  );
}
```

- [ ] **Step 2: Delete `ResponsePanel.tsx`**

```bash
git rm apps/desktop/src/components/ResponsePanel.tsx
```

- [ ] **Step 3: Typecheck**

Run: `pnpm -C apps/desktop run typecheck`
Expected: fails at this point only because `App.tsx` still imports `ResponsePanel` — that import is removed in Task 8. If Task 8 hasn't run yet, this is an expected transient failure; proceed to commit anyway since this task's own file is correct in isolation (verified by the `MessageList.tsx`-only lint would pass). If your workflow requires a green typecheck per commit, do Task 5 and Task 8 as one combined commit instead.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/components/MessageList.tsx apps/desktop/src/components/ResponsePanel.tsx
git commit -m "feat(desktop): add MessageList with Claude-style user bubbles"
```

---

## Task 6: `Sidebar` component (replaces `OverlayFooter`)

**Files:**
- Create: `apps/desktop/src/components/Sidebar.tsx`
- Delete: `apps/desktop/src/components/OverlayFooter.tsx`

**Interfaces:**
- Produces: `Sidebar({ onNewConversation: () => void; onOpenSettings: () => void; settingsActive: boolean; disabled: boolean })` — 48px-wide icon rail, mounted by `App.tsx` (Task 8) as the first flex child.
- Consumes: `@phosphor-icons/react` icons `Clock`, `Gear`, `Plus` (same package `OverlayFooter.tsx` already used for `Clock`/`Gear`).

- [ ] **Step 1: Create the component**

Create `apps/desktop/src/components/Sidebar.tsx`:

```tsx
import { Clock, Gear, Plus } from "@phosphor-icons/react";

// docs/12 §5.1/§16.1 rail — icon-only nav, no chat-history list (spec §2).
// Timeline stays disabled (unimplemented, spec §7); Settings opens the
// minimal panel from Task 7.
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
```

- [ ] **Step 2: Delete `OverlayFooter.tsx`**

```bash
git rm apps/desktop/src/components/OverlayFooter.tsx
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/Sidebar.tsx apps/desktop/src/components/OverlayFooter.tsx
git commit -m "feat(desktop): add icon-rail Sidebar replacing OverlayFooter"
```

(Same transient-typecheck note as Task 5 — `App.tsx` still references the old footer until Task 8.)

---

## Task 7: `SettingsPanel` component (minimal — Theme only)

**Files:**
- Create: `apps/desktop/src/components/SettingsPanel.tsx`

**Interfaces:**
- Consumes: `ThemePreference` from `../lib/theme` (Task 2).
- Produces: `SettingsPanel({ preference: ThemePreference; onChange: (value: ThemePreference) => void; onClose: () => void })`.

- [ ] **Step 1: Create the component**

Create `apps/desktop/src/components/SettingsPanel.tsx`:

```tsx
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
```

- [ ] **Step 2: Typecheck**

Run: `pnpm -C apps/desktop run typecheck`
Expected: no errors (this file has no dependency on `App.tsx`).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/SettingsPanel.tsx
git commit -m "feat(desktop): add minimal Settings panel (Theme only)"
```

---

## Task 8: Wire it together in `App.tsx`

**Files:**
- Modify: `apps/desktop/src/App.tsx`

**Interfaces:**
- Consumes: `Sidebar` (Task 6), `MessageList` (Task 5), `SettingsPanel` (Task 7), `useTheme` (Task 2), `overlayReducer`/`initialOverlayState`/`Message` (Task 3), `ACTION_LABELS` (Task 4).
- Produces: the assembled overlay UI — this is the plan's integration point; no downstream task consumes `App.tsx`.

- [ ] **Step 1: Replace `apps/desktop/src/App.tsx`**

```tsx
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
```

- [ ] **Step 2: Typecheck and run the full test suite**

Run: `pnpm -C apps/desktop run typecheck && pnpm -C apps/desktop test`
Expected: typecheck clean; all Vitest suites pass (Tasks 2 and 3's tests plus any pre-existing ones).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(desktop): wire sidebar, message list, and settings into App"
```

---

## Task 9: Manual verification pass

No component-test infra exists in this repo (Tech Stack note above), so this task is the functional verification for Tasks 5-8's presentational wiring — required before calling the redesign done (CLAUDE.md: UI changes must be exercised in a browser/dev server before completion).

- [ ] **Step 1: Start the dev server and open the overlay**

Run (or via the Browser-pane `preview_start`/`preview_start name` workflow if driving it interactively): `pnpm -C apps/desktop dev`

Open the served URL. Confirm: dark theme by default (no `data-theme` override → OS default), rail visible on the left with `+` / clock / gear icons, input bar at the bottom with quick actions directly above it, context indicator at the top.

- [ ] **Step 2: Multi-turn conversation**

Type a message, press Enter. Confirm: it appears as a right-aligned bubble, an assistant response streams in below as plain text, the input clears after submit. Type a second message. Confirm: both turns remain visible, list auto-scrolls to the newest message.

- [ ] **Step 3: New conversation**

Click the `+` icon in the rail. Confirm: the message list clears, input is refocused.

- [ ] **Step 4: Theme toggle**

Click the gear icon. Confirm: the Settings panel replaces the chat view, with a single Theme dropdown. Switch it to "Light". Confirm: background/text/accent colors flip to the light token set from Task 1 across rail, bubbles, input, and buttons. Switch to "Dark", then "System" — confirm each applies. Click "Back". Confirm: the chat view returns with the conversation still intact. Reload the page — confirm the last explicit choice persisted (via `localStorage`).

- [ ] **Step 5: Contrast check**

Using a WCAG contrast checker, verify `--color-accent` against `--color-bg-primary` for both themes wherever accent is used as *text* (not just icon fills/button backgrounds) meets 4.5:1 (docs/12 §13). If a pairing fails, adjust only that token's hex in `index.css` (Task 1) and re-verify — do not change the hue direction agreed in the spec.

- [ ] **Step 6: Resize check**

Resize the window down to `minWidth: 480` (docs/12 §5.3). Confirm the rail stays usable and the input/quick-actions/context-indicator don't clip or overflow horizontally.

- [ ] **Step 7: Escape / cancel**

Submit a message, press Escape mid-stream. Confirm: the request cancels (no further chunks appear) and the overlay hides (existing `hideOverlay` behavior, unchanged).

No commit for this task — it's verification only. If any check fails, fix the relevant file from Tasks 1-8 and re-run this task's checklist before proceeding.

---

## Task 10: Update `docs/12_UI_UX.md`

**Files:**
- Modify: `docs/12_UI_UX.md`

- [ ] **Step 1: Update the sections that no longer match the shipped UI**

Edit `docs/12_UI_UX.md`:
- §5.1 layout ASCII diagram — replace with the rail + bottom-input layout (mirror the diagram in the design spec §3.1).
- §5.3 — background is no longer flat `#0F0F14` (now theme-dependent, see §12.1 update below); note the outer frame still has no border radius, but inner elements (bubbles, input, buttons) do.
- §9.1/§9.2 — mark General → Theme as implemented; leave other sections marked not-yet-built.
- §12.1 — replace the single dark color table with the two-theme table from the design spec §3.2 (both dark and light columns).
- §16.1 component tree — replace `OverlayFooter`/`ResponsePanel` with `Sidebar`/`MessageList`, add `SettingsPanel` under a new `SettingsView` overlay sub-view entry (in-overlay swap, not a separate window, per spec §3.4).

- [ ] **Step 2: Commit**

```bash
git add docs/12_UI_UX.md
git commit -m "docs: update UI/UX spec for Claude-style overlay redesign"
```
