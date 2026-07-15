//! UI Automation Extractor — `docs/05_Vision_Engine.md` §5.5. `walk()` +
//! `confidence()` ported verbatim from the validated
//! `spikes/SP-01-uia-coverage/src/main.rs` (86% coverage, p95 ~110ms
//! non-Office — `benchmarks/BASELINE.md`).
//!
//! Must be constructed and used from a thread with COM initialized STA
//! (ADR-0008) — UIA requires it. Not unit-testable beyond `confidence()`
//! (exercised via `examples/vision_smoke.rs`).

use std::time::Instant;

use uiautomation::controls::ControlType;
use uiautomation::patterns::{UITextPattern, UIValuePattern};
use uiautomation::types::Handle;
use uiautomation::{UIAutomation, UIElement};
use windows::Win32::Foundation::HWND;

use contexa_core::{ContexaError, Result};

use crate::types::UiaResult;

const MAX_DEPTH: usize = 20;
const MAX_ELEMENTS: usize = 2000;
// mirrors production's context char budget — early-stop once filled.
const ENOUGH_CHARS: usize = 2000;

pub struct UiaExtractor {
    automation: UIAutomation,
}

impl UiaExtractor {
    /// # Errors
    /// Returns an error if a `IUIAutomation` COM instance can't be created
    /// (e.g. called off an STA thread).
    pub fn new() -> Result<Self> {
        let automation = UIAutomation::new().map_err(|e| ContexaError::CaptureFailed {
            reason: e.to_string(),
        })?;
        Ok(Self { automation })
    }

    /// # Errors
    /// Returns an error if `hwnd` has no accessible UIA root element.
    pub fn extract_text(&self, hwnd: isize) -> Result<UiaResult> {
        let win = self
            .automation
            .element_from_handle(Handle::from(HWND(hwnd as *mut std::ffi::c_void)))
            .map_err(|e| ContexaError::CaptureFailed {
                reason: e.to_string(),
            })?;

        let t = Instant::now();
        let mut out = String::new();
        let mut seen = 0usize;
        let mut tp_chars = 0usize;
        let mut max_depth = 0usize;
        walk(
            &self.automation,
            &win,
            0,
            &mut seen,
            &mut out,
            &mut tp_chars,
            &mut max_depth,
        );
        let duration_ms = u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX);

        Ok(UiaResult {
            confidence: confidence(out.len(), tp_chars),
            text: out,
            element_count: u32::try_from(seen).unwrap_or(u32::MAX),
            tree_depth: u32::try_from(max_depth).unwrap_or(u32::MAX),
            duration_ms,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    auto: &UIAutomation,
    el: &UIElement,
    depth: usize,
    seen: &mut usize,
    out: &mut String,
    tp_chars: &mut usize,
    max_depth: &mut usize,
) {
    if depth > MAX_DEPTH || *seen > MAX_ELEMENTS || out.len() >= ENOUGH_CHARS {
        return;
    }
    *seen += 1;
    *max_depth = (*max_depth).max(depth);

    if let Ok(name) = el.get_name() {
        if !name.is_empty() {
            out.push_str(&name);
            out.push('\n');
        }
    }
    // pattern queries are COM QueryInterface calls — only pay for text-bearing controls
    let ct = el.get_control_type().ok();
    if matches!(
        ct,
        Some(ControlType::Edit | ControlType::ComboBox | ControlType::Document)
    ) {
        if let Ok(vp) = el.get_pattern::<UIValuePattern>() {
            if let Ok(v) = vp.get_value() {
                if !v.is_empty() {
                    out.push_str(&v);
                    out.push('\n');
                }
            }
        }
    }
    // TextPattern on document/edit controls — the high-value extraction path
    if matches!(ct, Some(ControlType::Document | ControlType::Edit)) {
        if let Ok(tp) = el.get_pattern::<UITextPattern>() {
            if let Ok(range) = tp.get_document_range() {
                if let Ok(text) = range.get_text(65536) {
                    *tp_chars += text.len();
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
    }

    if let Ok(walker) = auto.create_tree_walker() {
        if let Ok(mut child) = walker.get_first_child(el) {
            loop {
                walk(auto, &child, depth + 1, seen, out, tp_chars, max_depth);
                match walker.get_next_sibling(&child) {
                    Ok(next) => child = next,
                    Err(_) => break,
                }
                if *seen > MAX_ELEMENTS {
                    break;
                }
            }
        }
    }
}

/// Heuristic confidence: 1.0 = rich text via `TextPattern` or large harvest;
/// 0.85 = solid text without `TextPattern`; 0.5 = fragments; 0.0 = nothing.
/// Docs/05 §5.5 bands: High >0.8 proceed, Medium 0.5-0.8, Low <0.5 -> OCR.
#[must_use]
pub fn confidence(chars: usize, text_pattern_chars: usize) -> f32 {
    if text_pattern_chars > 50 || chars > 1000 {
        1.0
    } else if chars > 300 {
        0.85
    } else if chars > 50 {
        0.5
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_text_pattern_is_high_confidence() {
        assert_eq!(confidence(200, 60), 1.0);
    }

    #[test]
    fn large_harvest_without_text_pattern_is_high_confidence() {
        assert_eq!(confidence(1200, 0), 1.0);
    }

    #[test]
    fn solid_text_without_text_pattern_is_medium_high() {
        assert_eq!(confidence(500, 0), 0.85);
    }

    #[test]
    fn fragments_are_medium_low() {
        assert_eq!(confidence(100, 0), 0.5);
    }

    #[test]
    fn nothing_is_zero_confidence() {
        assert_eq!(confidence(10, 0), 0.0);
    }
}
