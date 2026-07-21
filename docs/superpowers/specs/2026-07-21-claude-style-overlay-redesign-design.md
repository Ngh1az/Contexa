# Design: Claude-style Overlay Redesign

**Date:** 2026-07-21
**Status:** Draft — pending user review
**Scope:** `apps/desktop/src` (React overlay UI only). No Rust/Tauri backend changes.

## 1. Problem

The current overlay (docs/12 §5, `App.tsx`) is single-shot: one query, one
response, reset every time the overlay regains focus (`onOverlayFocus` →
`dispatch({ type: "reset" })` in `App.tsx:81-89`, `overlayState.ts` holds a
single `response: string`, not a list). Footer nav (`OverlayFooter.tsx`) is
two disabled buttons. No theme option exists — dark only, tokens hardcoded
in `index.css` (`--color-bg-primary: #0f0f14`, accent `#6c5ce7`).

The user wants the overlay's visual language and interaction model to match
Claude.ai: warm cream/dark palette, a left icon-rail, multi-turn chat with
user-message bubbles, input pinned to the bottom, and a light/dark toggle.

This changes both **visuals** (color tokens) and **behavior** (multi-turn
state, new components) — confirmed with the user via the brainstorming
visual-companion flow (mockups: `.superpowers/brainstorm/558-1784610034/content/`).

## 2. Decisions (from brainstorming session)

| Question | Decision |
|---|---|
| Redesign depth | Full layout change (sidebar + bubbles), not just recolor |
| Theme | Both light and dark, user-togglable |
| Sidebar content | Icon-rail only — New, Timeline, Settings (no chat-history list) |
| Multi-turn | Yes — conversation persists within one overlay session, resets on reopen (same reset trigger as today) |
| Input position | Bottom-pinned (Claude.ai style), scroll region above it |
| Theme toggle location | Settings → General (new minimal Settings panel, this scope only) |
| Settings panel scope | Build only General → Theme; other sections (AI Provider, Capture, …) stay out of scope, not touched |

## 3. Visual Design

### 3.1 Layout

```
┌────┬──────────────────────────────────────────┐
│ +  │  📄 VS Code — main.rs        (context)    │
│ 🕐 │──────────────────────────────────────────│
│ ⚙  │  (message list, scrollable)               │
│    │   ┌───────────────────────────┐           │
│    │   │              user bubble ►│           │
│    │   └───────────────────────────┘           │
│    │   assistant text, no bubble               │
│    │                                            │
│    │──────────────────────────────────────────│
│    │  [Explain] [Summarize] [Translate] [🔍]    │
│    │  ┌──────────────────────────────────────┐ │
│    │  │ Ask anything about your screen…       │ │
│    │  └──────────────────────────────────────┘ │
└────┴──────────────────────────────────────────┘
```

Rail is 48px wide, icon-only, three items: **New** (`+`, accent-filled
square, starts a fresh conversation — replaces the implicit reset),
**Timeline** (clock icon), **Settings** (gear icon). Timeline keeps its
current "coming soon" disabled state (out of scope). Settings becomes live
but opens only the minimal panel from §3.4.

User messages render as a right-aligned bubble (`bg-tertiary`, rounded
`12px 12px 2px 12px`). Assistant responses stay plain text, no bubble —
preserves the existing "Claude.ai/Codex-CLI minimal" direction already
noted in `ResponsePanel.tsx:10`. Context indicator stays above the message
list (unchanged position/behavior). Quick actions move to sit directly
above the input bar, both pinned to the bottom.

### 3.2 Color Tokens

Replace `index.css` `@theme` block. Two token sets, switched by a
`data-theme` attribute on `<html>` (Tailwind v4 supports this via
`@theme` + a `[data-theme="light"]` override block — no new dependency).

| Token | Dark | Light |
|---|---|---|
| `--color-bg-primary` | `#262624` | `#faf9f5` |
| `--color-bg-secondary` (rail, input) | `#1e1e1c` | `#f4f3ee` |
| `--color-bg-tertiary` (user bubble) | `#3a3a37` | `#e8e6dc` |
| `--color-text-primary` | `#f0eee6` | `#1f1e1d` |
| `--color-text-secondary` | `#8f8d86` | `#6b6a66` |
| `--color-accent` | `#d97757` | `#c15f3c` |
| `--color-accent-hover` | `#e08966` | `#ad5434` |
| `--color-success` | `#00b894` | `#00966f` |
| `--color-warning` | `#fdcb6e` | `#b8860b` |
| `--color-error` | `#e17055` | `#c44f36` |
| `--color-border` | `#3a3a37` | `#e5e3da` |

Mockups approved by user; exact values may shift slightly during
implementation if a contrast check (§6) fails AA for a specific
text/background pairing — the hue direction (warm terracotta accent, warm
cream/graphite neutrals) is what's locked in, not the literal hex.

### 3.3 Message List Behavior

- Auto-scrolls to bottom on new message / streaming chunk (unless user has
  scrolled up — then show a "jump to latest" affordance, standard chat
  pattern, small addition not previously specced).
- Streaming still appends tokens to the *last* assistant message, same
  mechanism as today's `chunk` reducer case — just addressed to
  `messages[messages.length - 1]` instead of a single `response` field.

### 3.4 Settings Panel (minimal)

New `SettingsPanel` component, opened as an in-overlay view swap (same
pattern the component tree already reserves for `TimelinePanel`, docs/12
§16.1) — not a separate OS window, to keep this a one-file, low-effort
addition. Content: a single "General" section with one control, "Theme:
[System / Light / Dark]". No sidebar-within-settings, no other sections —
everything else in docs/12 §9.1 stays a future item.

## 4. State Changes

### 4.1 `overlayState.ts`

```ts
export interface Message {
  id: string;          // requestId for the in-flight one; uuid otherwise
  role: "user" | "assistant";
  content: string;
}

export interface OverlayState {
  phase: OverlayPhase;
  requestId: string | null;
  messages: Message[];
  error: string | null;
}
```

`submit` appends a user message + a placeholder assistant message;
`chunk`/`complete` mutate the last assistant message by `requestId` match
(same guard logic as today, just indexed differently). `error`/`rejected`
surface into the top-level `error` banner only — no per-message error
field, since nothing renders one (avoids unused state). `reset` clears
`messages` back to `[]` (still fired from `onOverlayFocus`, per the
"resets on reopen" decision in §2). The rail's `+` button dispatches the
same `reset` action mid-session.

### 4.2 Theme state

Not part of `overlayReducer` — separate small hook (`useTheme()`) backed by
`localStorage` (`contexa-theme`, values `system|light|dark`). No Tauri
store plugin needed (none is installed; `localStorage` in the WebView
already persists across app restarts, same as any browser profile) —
picking the smallest tool that solves it per project convention. `system`
resolves via `window.matchMedia("(prefers-color-scheme: dark)")` and
listens for OS changes when in that mode.

## 5. Component Changes

| File | Change |
|---|---|
| `src/components/Sidebar.tsx` | **New.** Icon rail: New / Timeline / Settings buttons |
| `src/components/MessageList.tsx` | **New.** Replaces `ResponsePanel`'s role — renders `Message[]`, user bubbles + assistant plain text, auto-scroll |
| `src/components/SettingsPanel.tsx` | **New.** Theme selector only |
| `src/components/ResponsePanel.tsx` | Removed — logic absorbed into `MessageList` (single message rendering becomes a sub-component `MessageBubble` inside it) |
| `src/components/OverlayFooter.tsx` | Removed — Timeline/Settings move into `Sidebar` |
| `src/lib/overlayState.ts` | `response: string` → `messages: Message[]` (see §4.1) |
| `src/lib/theme.ts` | **New.** `useTheme()` hook (see §4.2) |
| `src/App.tsx` | Layout restructured to `flex-row` (Sidebar + main column); main column reorders to context-indicator → message-list (flex-1, scroll) → quick-actions → input (both bottom) |
| `src/index.css` | Token table from §3.2 |

## 6. Testing

- Extend `src/lib/overlayState.test.ts` (existing assert-based style, no
  new framework) for the `messages` array: submit appends user+placeholder,
  chunk/complete/error address the right message by requestId, reset
  clears the list, a stale requestId chunk is ignored.
- New `src/lib/theme.test.ts`: `system` resolves from `matchMedia`,
  explicit `light`/`dark` overrides it and persists to `localStorage`.
- Manual verification in the dev overlay (per CLAUDE.md UI rule): open
  overlay, multi-turn a conversation, toggle theme from Settings, resize
  to confirm rail + bottom input hold up at `minWidth: 480`.
- Contrast check: run the new accent/bg pairs from §3.2 through a WCAG
  contrast calculator for the specific places `--color-accent` is used as
  *text* (not just fills/icons) — adjust that one token if it's under
  4.5:1, per docs/12 §13.

## 7. Out of Scope

- Chat-history list in the sidebar (would need Memory Engine query
  wiring — separate spec).
- Full Settings window (AI Provider, Capture, Memory, Search, Privacy,
  MCP, About sections) — only General/Theme ships here.
- Persisting conversation across overlay close/reopen.
- Timeline panel implementation (still the existing disabled stub).

## 8. Docs to Update Post-Implementation

- `docs/12_UI_UX.md`: §5.1 layout diagram, §5.3 background/border-radius
  (no longer flat `#0F0F14`, still no radius on outer frame — inner
  elements gain radius), §12.1 color table, §16.1 component tree,
  §9.1/§9.2 (Settings now partially implemented).
