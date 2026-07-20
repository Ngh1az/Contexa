# UI / UX Design

**Project:** Contexa — AI Context Platform  
**Version:** 1.2  
**Status:** Reviewed  
**Last Updated:** 2026-07-18

---

## 1. Overview

The Contexa user interface consists of three primary surfaces: the **Overlay** (activated by `Alt + Space`), the **Settings** window, and the **System Tray**. The design prioritizes speed, minimal disruption, and context-aware interactions.

---

## 2. Goals

1. Open overlay within 200ms of hotkey press
2. Enable one-keystroke access to common AI actions
3. Display AI responses with streaming for perceived instant feedback
4. Provide timeline and settings without leaving the overlay
5. Maintain a clean, modern aesthetic that feels native to Windows

---

## 3. Responsibilities

| Surface | Responsibility |
|---------|----------------|
| Overlay | Primary AI interaction; chat, quick actions, streaming responses |
| Settings | Configuration for LLM, capture, privacy, MCP, hotkeys |
| System Tray | Status indicator, quick access, quit |
| Timeline View | Browse chronological work history |
| Onboarding | First-run setup wizard |

---

## 4. Architecture

```mermaid
flowchart TB
    subgraph UI["React UI (Tauri WebView)"]
        Overlay[Overlay Window]
        Settings[Settings Window]
        Timeline[Timeline View]
        Onboarding[Onboarding Wizard]
    end

    subgraph Components
        Chat[Chat Panel]
        QA[Quick Actions]
        SR[Streaming Response]
        CI[Context Indicator]
        SB[Search Bar]
    end

    Overlay --> Chat
    Overlay --> QA
    Overlay --> SR
    Overlay --> CI
    Overlay --> SB
    Overlay --> Timeline
```

---

## 5. Overlay Design

### 5.1 Layout

```
┌─────────────────────────────────────────────────┐
│  🔍  Ask anything about your screen...          │ ← Input bar
├─────────────────────────────────────────────────┤
│  [Explain] [Summarize] [Translate] [Search]       │ ← Quick actions
├─────────────────────────────────────────────────┤
│                                                 │
│  📄 VS Code — main.rs                    ← Context indicator
│  ─────────────────────────────────────────────│
│                                                 │
│  AI Response area with streaming text...        │ ← Response panel
│                                                 │
│                                                 │
├─────────────────────────────────────────────────┤
│  📋 Timeline    ⚙️ Settings    ✕ Close           │ ← Footer actions
└─────────────────────────────────────────────────┘
```

### 5.2 Overlay States

```mermaid
stateDiagram-v2
    [*] --> Hidden
    Hidden --> Input: Alt+Space
    Input --> Processing: Submit / Quick Action
    Processing --> Streaming: First token received
    Streaming --> Input: Response complete
    Input --> Hidden: Escape
    Processing --> Hidden: Escape (cancel)
    Streaming --> Hidden: Escape
    Input --> Timeline: Click Timeline
    Timeline --> Input: Back
    Input --> Settings: Click Settings
    Settings --> Input: Back
```

### 5.3 Dimensions & Positioning

**v1.2 pivot:** the overlay is a regular OS-decorated window (native title
bar — minimize/maximize/close, resizable, draggable), not a frameless
Spotlight/Raycast-style popup. Reason: user testing of the v1.1 frameless
popup (fixed 600×500, no decorations, always-on-top, transparent) found
users expected standard window controls, consistent with reference layouts
(Claude.ai, Cursor, Codex CLI) — those are persistent app surfaces, not
transient popups. `Alt+Space` still toggles show/hide; closing via the
native title-bar X hides rather than quits (background capture keeps
running — docs/16 §7.1; quit is tray-menu only, §10).

| Property | Value |
|----------|-------|
| Initial size | 900×640px (`minWidth` 480, `minHeight` 360) |
| Resizable | Yes |
| Decorations | Native OS title bar (minimize/maximize/close) |
| Position | Centered on first launch; OS remembers position while running |
| Background | Solid `--bg-primary` (`#0F0F14`, no transparency) |
| Border radius | None (edge-to-edge, matches native frame) |
| Z-order | Normal (not always-on-top) |
| Animation | Content fade in 150ms, slide up 10px on show |

### 5.4 Context Indicator

Shows the current desktop context at a glance:

```
📄 VS Code — src/main.rs
🌐 Chrome — github.com/contexa
📊 Excel — Q3_Report.xlsx
```

Clicking expands to show full context details (app, URL, visible text preview).

---

## 6. Quick Actions

| Action | Icon | Shortcut | Behavior |
|--------|------|----------|----------|
| Explain | 💡 | `E` | Explain current content/selection |
| Summarize | 📝 | `S` | Summarize visible content |
| Translate | 🌐 | `T` | Translate selection (language picker) |
| Search | 🔍 | `/` | Search with context + web |

Quick actions send predefined requests to the Orchestrator without requiring typed input.

---

## 7. Chat Interaction

### 7.1 Input Bar

- Placeholder: "Ask anything about your screen..."
- Supports multi-line input (`Shift + Enter`)
- Submit: `Enter`
- Shows context indicator badge when context is available
- Auto-focus on overlay open

### 7.2 Streaming Response

```mermaid
sequenceDiagram
    participant UI as Overlay UI
    participant Tauri as Tauri IPC
    participant AO as Orchestrator

    UI->>Tauri: handle_request({ action: "chat", query })
    Tauri-->>UI: { request_id, status: "accepted" }
    UI->>UI: Show loading indicator
    
    loop Streaming
        Tauri-->>UI: ai-chunk { content, done: false }
        UI->>UI: Append token to response
    end
    
    Tauri-->>UI: ai-complete { total_tokens, latency_ms }
    UI->>UI: Show completion indicator
```

### 7.3 Response Rendering

- Markdown rendering (headings, lists, code blocks, links)
- Code blocks with syntax highlighting and copy button
- Source citations from memory/search displayed as footnotes
- "Copy" button on hover for response blocks

---

## 8. Timeline View

### 8.1 Layout

```
┌─────────────────────────────────────────────────┐
│  ← Back          Timeline — Today                  │
├─────────────────────────────────────────────────┤
│  09:15  📄 Opened VS Code — main.rs       45m   │
│  10:00  🌐 Chrome — GitHub PR review       30m   │
│  10:30  📄 VS Code — fix auth module       1h    │
│  11:30  📊 Excel — Q3 budget review        20m   │
│  11:50  💬 "Explain OAuth flow"                   │
│  12:00  🌐 Chrome — OAuth documentation    45m   │
├─────────────────────────────────────────────────┤
│  [Today] [Yesterday] [This Week] [Custom]        │
└─────────────────────────────────────────────────┘
```

### 8.2 Interactions

- Click event → expand to show context snapshot details
- Filter by application type
- Search within timeline
- Date range selector

---

## 9. Settings

### 9.1 Sections

| Section | Settings |
|---------|----------|
| **General** | Hotkey, auto-start, language |
| **AI Provider** | Provider, model, API key, temperature, max tokens |
| **Capture** | Enable/disable, excluded apps, excluded URLs |
| **Memory** | Retention period, embedding model, clear data |
| **Search** | Enable/disable, provider, API key |
| **Privacy** | Send context to cloud, data export, delete all |
| **MCP** | Server status, generate/revoke tokens, connected clients |
| **About** | Version, license, documentation links |

### 9.2 Settings Layout

```
┌──────────┬──────────────────────────────────────┐
│ General  │  Overlay Hotkey                      │
│ AI       │  ┌─────────────────┐                 │
│ Capture  │  │ Alt + Space     │  [Change]       │
│ Memory   │  └─────────────────┘                 │
│ Search   │                                      │
│ Privacy  │  ☐ Start Contexa on login            │
│ MCP      │                                      │
│ About    │  Language: [English ▾]               │
│          │                                      │
└──────────┴──────────────────────────────────────┘
```

---

## 10. System Tray

| State | Icon | Tooltip |
|-------|------|---------|
| Active | Green dot | "Contexa — Active" |
| Paused | Yellow dot | "Contexa — Capture paused" |
| Error | Red dot | "Contexa — Error" |

**Tray menu:**
- Open Overlay (`Alt+Space`)
- Pause/Resume Capture
- Settings
- Timeline
- Quit

---

## 11. Onboarding Flow

```mermaid
flowchart LR
    A[Welcome] --> B[Privacy Consent]
    B --> C[Configure AI Provider]
    C --> D[Set Hotkey]
    D --> E[Capture Preferences]
    E --> F[Ready]
```

| Step | Content |
|------|---------|
| Welcome | "Contexa gives AI real-time awareness of your desktop" |
| Privacy | Explain what is captured; opt-in for cloud LLM and search |
| AI Provider | Select provider; enter API key or configure Ollama |
| Hotkey | Confirm `Alt+Space` or customize |
| Capture | Default exclusions; option to add apps |
| Ready | "Press Alt+Space anywhere to try it" |

---

## 12. Design System

### 12.1 Colors (Dark Theme)

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-primary` | `#0F0F14` | Overlay background |
| `--bg-secondary` | `#1A1A24` | Cards, input fields |
| `--text-primary` | `#E8E8ED` | Body text |
| `--text-secondary` | `#9898A4` | Labels, hints |
| `--accent` | `#6C5CE7` | Buttons, links, focus |
| `--accent-hover` | `#7D6FF0` | Hover states |
| `--success` | `#00B894` | Active status |
| `--warning` | `#FDCB6E` | Paused status |
| `--error` | `#E17055` | Error states |
| `--border` | `#2A2A36` | Borders, dividers |

### 12.2 Typography

| Element | Font | Size | Weight |
|---------|------|------|--------|
| Heading | Inter | 16px | 600 |
| Body | Inter | 14px | 400 |
| Small | Inter | 12px | 400 |
| Code | JetBrains Mono | 13px | 400 |
| Input | Inter | 14px | 400 |

### 12.3 Spacing

Base unit: 4px. Common values: 4, 8, 12, 16, 24, 32.

---

## 13. Accessibility

- Full keyboard navigation (Tab, Enter, Escape)
- ARIA labels on all interactive elements
- Focus indicators visible on all focusable elements
- Color contrast ratio ≥ 4.5:1 (WCAG AA)
- Screen reader announces context changes and AI responses
- Reduced motion option respects `prefers-reduced-motion`

---

## 14. Performance

| Metric | Target |
|--------|--------|
| Overlay open animation | 150ms |
| Input focus | Immediate |
| First render | < 100ms |
| Streaming update | < 16ms per token (60fps) |
| Timeline scroll | 60fps with 1000+ events |

---

## 15. Security

- API keys displayed as masked; never shown in full after initial entry
- "Delete all data" requires confirmation dialog with type-to-confirm
- MCP tokens shown once; copy-to-clipboard only
- No sensitive data in overlay screenshots or logs

---

---

## 16. Component Specifications

### 16.1 Component Tree

```
App
├── SystemTray (always running)
├── OverlayWindow (on hotkey)
│   ├── OverlayHeader
│   │   ├── SearchInput
│   │   └── ContextBadge
│   ├── QuickActionBar
│   │   ├── ExplainButton
│   │   ├── SummarizeButton
│   │   ├── TranslateButton
│   │   └── SearchButton
│   ├── ResponsePanel
│   │   ├── StreamingText (markdown)
│   │   ├── SourceCitations
│   │   └── LoadingIndicator
│   └── OverlayFooter
│       ├── TimelineToggle
│       └── SettingsToggle
├── TimelinePanel (overlay sub-view)
│   ├── DateFilter
│   ├── TimelineList (virtualized)
│   └── EventDetail
├── SettingsWindow (separate window)
│   ├── SettingsSidebar
│   └── SettingsContent (per section)
└── OnboardingWizard (first-run modal)
```

### 16.2 Interaction Matrix

| User Action | Key/Input | System Response | Latency Target |
|-------------|-----------|-----------------|----------------|
| Open overlay | `Alt+Space` | Show overlay, focus input, load context badge | < 200 ms |
| Submit query | `Enter` | Send to orchestrator, show loading, stream response | < 50 ms accept |
| Quick explain | `E` or click | Trigger explain action on current context | < 50 ms accept |
| Quick summarize | `S` or click | Trigger summarize action | < 50 ms accept |
| Quick translate | `T` or click | Show language picker, then translate | < 50 ms accept |
| Cancel / close | `Escape` | Hide overlay, cancel in-flight request | < 100 ms |
| Open timeline | Click footer | Switch to timeline sub-view | < 50 ms |
| Open settings | Click footer | Open settings window | < 200 ms |
| Copy response | Click copy icon | Copy markdown to clipboard | < 50 ms |
| Navigate timeline | Arrow keys | Scroll timeline list | 60 fps |
| Pause capture | Tray menu | Yellow indicator, stop context updates | < 100 ms |

### 16.3 Responsive Behavior

| Display | Overlay Width | Font Scale | Notes |
|---------|--------------|------------|-------|
| 1080p | 600 px | 1.0× | Default |
| 1440p | 640 px | 1.0× | Slightly wider |
| 4K (150% DPI) | 600 px logical | 1.0× | Tauri handles DPI scaling |
| 4K (200% DPI) | 600 px logical | 1.0× | Test touch targets ≥ 44px |

### 16.4 Error States

| Error | UI Display | Recovery |
|-------|-----------|----------|
| LLM provider down | "AI provider unavailable. [Check Settings]" | Retry button; suggest fallback |
| No context captured | Context badge: "No active window" | Explain limitation; suggest focus app |
| OCR failed | Response uses UIA text only | Transparent to user unless no text |
| Search disabled | "Web search is off. [Enable in Settings]" | Link to settings |
| MCP token invalid | Settings → MCP: "Token revoked" | Regenerate token |
| Rate limited | "Too many requests. Try again in 30s." | Auto-retry countdown |

### 16.5 Animation Specifications

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Overlay fade in | 150 ms | ease-out | Hotkey press |
| Overlay slide up | 150 ms | ease-out | Hotkey press (10px translateY) |
| Overlay fade out | 100 ms | ease-in | Escape |
| Token stream | — | — | Append text, no per-token animation |
| Loading pulse | 1000 ms | ease-in-out | Waiting for first token |
| Context badge update | 200 ms | ease | Context change event |

---

## 17. Future Expansion

- **Light theme** option
- **Custom themes** via CSS variables
- **Widget mode** — small persistent context pill
- **Voice input** — microphone button in overlay
- **Multi-monitor** — overlay appears on active monitor
- **Notification toasts** — proactive context suggestions

---

## 18. Best Practices

- Use TailwindCSS utility classes; avoid custom CSS where possible
- Implement virtual scrolling for timeline (react-window)
- Debounce input for search-as-you-type
- Preload overlay WebView on app startup for instant open
- Test overlay on 1080p, 1440p, and 4K displays

---

## 19. References

- [01_Software_Requirements_Specification.md](./01_Software_Requirements_Specification.md)
- [03_API_Interface_Specification.md](./03_API_Interface_Specification.md)
- [TailwindCSS](https://tailwindcss.com/)
- [Tauri Window API](https://tauri.app/reference/javascript/api/namespacewindow/)
