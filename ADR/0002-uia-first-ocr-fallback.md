# ADR-0002: UI Automation First, OCR Fallback

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team

---

## Context

Contexa needs to extract text content from the user's active window to build context. Two primary approaches exist:

1. **UI Automation (UIA)** — Access the accessibility tree of UI elements
2. **OCR (Optical Character Recognition)** — Capture screenshot and extract text from pixels

OCR is CPU-intensive, especially when run continuously on the entire screen. UIA provides structured text access with minimal resource usage but may not work for all applications.

## Decision

Use **UI Automation as the primary text extraction method**. Use **OCR only as a targeted fallback** when UIA confidence is below 0.5, and only on changed screen regions. Never continuously OCR the entire screen.

## Rationale

| Factor | UIA First | OCR First | Hybrid (equal) |
|--------|-----------|-----------|----------------|
| CPU usage | < 1% | 10-30% | 5-15% |
| Text accuracy | 95%+ for standard apps | 90%+ | 95%+ |
| Structured data | Yes (elements, types) | No (flat text) | Partial |
| Custom-rendered UIs | Poor | Good | Good |
| Battery impact | Minimal | Significant | Moderate |

UIA works well for the majority of applications users interact with daily (browsers, Office, IDEs, system dialogs). OCR is reserved for applications where UIA fails (some Electron apps, image-based PDFs, games).

## Consequences

**Positive:**
- Background CPU stays under 5% target
- Structured UI element data available for enrichment
- Password fields detectable via `IsPassword` property
- Frame differencing + region hashing further reduce work

**Negative:**
- Some applications will have lower context quality until app-specific enrichers are built
- OCR latency (500ms) when fallback is triggered
- UIA tree depth limited to 20 levels for performance

## Implementation

```
Capture → UIA Extract → Confidence Check
                              ↓ < 0.5
                         OCR (changed regions only, max 2/sec)
```

## References

- [05_Vision_Engine.md](../docs/05_Vision_Engine.md)
- [17_Performance_Optimization.md](../docs/17_Performance_Optimization.md)
