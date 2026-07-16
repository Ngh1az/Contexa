//! Snapshot Assembler — `docs/06_Context_Engine.md` §5.1. Transforms a
//! `VisionResult` into the flat `ContextSnapshot` (contexa-core's shape,
//! which already matches the `context_snapshots` DB row — see
//! `contexa-core/src/types.rs`'s module doc comment for why this differs
//! from docs/06 §8's nested `WindowInfo`/`ApplicationInfo` sketch).

use std::collections::HashMap;

use uuid::Uuid;

use contexa_core::{CaptureMethod, ContextSnapshot};
use contexa_vision::{OcrResult, UiaResult, VisionResult};

// docs/06 §11: "visible_text truncated to 50,000 chars to prevent memory abuse"
const MAX_VISIBLE_TEXT_CHARS: usize = 50_000;

#[derive(Debug, Default)]
pub struct SnapshotAssembler;

impl SnapshotAssembler {
    #[must_use]
    pub fn assemble(&self, vision: VisionResult) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: vision.timestamp,
            window_title: vision.window_title,
            process_name: vision.process_name,
            process_id: i64::from(vision.process_id),
            hwnd: i64::try_from(vision.hwnd).ok(),
            url: None,           // filled by enrichers
            document_path: None, // filled by enrichers
            visible_text: merge_text(vision.uia_result.as_ref(), vision.ocr_result.as_ref()),
            selected_text: None, // Selection Tracker deferred (docs/06 §5.5)
            metadata: HashMap::new(),
            language: None, // filled by the language detector
            capture_method: resolve_capture_method(
                vision.uia_result.as_ref(),
                vision.ocr_result.as_ref(),
            ),
        }
    }
}

fn merge_text(uia: Option<&UiaResult>, ocr: Option<&OcrResult>) -> Option<String> {
    let merged = match (uia, ocr) {
        (Some(uia), None) => Some(uia.text.clone()),
        (None, Some(ocr)) => Some(ocr.text.clone()),
        (Some(uia), Some(ocr)) => Some(format!("{}\n{}", uia.text, ocr.text)),
        (None, None) => None,
    };
    merged.map(truncate_chars)
}

fn truncate_chars(text: String) -> String {
    if text.chars().count() > MAX_VISIBLE_TEXT_CHARS {
        text.chars().take(MAX_VISIBLE_TEXT_CHARS).collect()
    } else {
        text
    }
}

fn resolve_capture_method(uia: Option<&UiaResult>, ocr: Option<&OcrResult>) -> CaptureMethod {
    match (uia.is_some(), ocr.is_some()) {
        (true, true) => CaptureMethod::Hybrid,
        (false, true) => CaptureMethod::Ocr,
        // (true, false): real UIA result. (false, false): no better signal — safe default (ADR-0002).
        _ => CaptureMethod::Uia,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn uia(text: &str) -> UiaResult {
        UiaResult {
            text: text.to_string(),
            element_count: 1,
            tree_depth: 1,
            confidence: 1.0,
            duration_ms: 1,
        }
    }

    fn ocr(text: &str) -> OcrResult {
        OcrResult {
            text: text.to_string(),
            regions: Vec::new(),
            confidence: 1.0,
            cached: false,
            duration_ms: 1,
        }
    }

    fn vision(uia_result: Option<UiaResult>, ocr_result: Option<OcrResult>) -> VisionResult {
        VisionResult {
            hwnd: 42,
            window_title: "Notepad".to_string(),
            process_name: "notepad.exe".to_string(),
            process_id: 1234,
            frame_hash: [0; 4],
            changed_regions: Vec::new(),
            uia_result,
            ocr_result,
            capture_method: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn maps_window_and_process_fields() {
        let snapshot = SnapshotAssembler.assemble(vision(None, None));
        assert_eq!(snapshot.window_title, "Notepad");
        assert_eq!(snapshot.process_name, "notepad.exe");
        assert_eq!(snapshot.process_id, 1234);
        assert_eq!(snapshot.hwnd, Some(42));
        assert!(snapshot.url.is_none());
        assert!(snapshot.document_path.is_none());
        assert!(snapshot.selected_text.is_none());
        assert!(snapshot.language.is_none());
    }

    #[test]
    fn merges_uia_only() {
        let snapshot = SnapshotAssembler.assemble(vision(Some(uia("hello")), None));
        assert_eq!(snapshot.visible_text.as_deref(), Some("hello"));
        assert_eq!(snapshot.capture_method, CaptureMethod::Uia);
    }

    #[test]
    fn merges_ocr_only() {
        let snapshot = SnapshotAssembler.assemble(vision(None, Some(ocr("world"))));
        assert_eq!(snapshot.visible_text.as_deref(), Some("world"));
        assert_eq!(snapshot.capture_method, CaptureMethod::Ocr);
    }

    #[test]
    fn merges_uia_and_ocr_as_hybrid() {
        let snapshot = SnapshotAssembler.assemble(vision(Some(uia("hello")), Some(ocr("world"))));
        assert_eq!(snapshot.visible_text.as_deref(), Some("hello\nworld"));
        assert_eq!(snapshot.capture_method, CaptureMethod::Hybrid);
    }

    #[test]
    fn no_text_when_neither_source_present() {
        let snapshot = SnapshotAssembler.assemble(vision(None, None));
        assert!(snapshot.visible_text.is_none());
        assert_eq!(snapshot.capture_method, CaptureMethod::Uia);
    }

    #[test]
    fn truncates_visible_text_to_50k_chars() {
        let long_text = "a".repeat(60_000);
        let snapshot = SnapshotAssembler.assemble(vision(Some(uia(&long_text)), None));
        assert_eq!(snapshot.visible_text.map(|t| t.chars().count()), Some(50_000));
    }
}
