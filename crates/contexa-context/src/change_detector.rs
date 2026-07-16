//! Change Detector — `docs/06_Context_Engine.md` §5.3. Prevents event
//! flooding by reporting a meaningful change only when the window, URL,
//! document, selection, or visible text has meaningfully drifted from the
//! last observed snapshot.

use contexa_core::ContextSnapshot;

// docs/06 §5.3: "significant if > 10% text changed or > 100 chars different".
// Measured as a length-delta ratio, not a real content diff: change
// detection has a <1ms budget (docs/06 §10) and `visible_text` can be up to
// 50,000 chars (docs/06 §11), so an O(n*m) diff isn't affordable here.
const SIGNIFICANT_CHAR_DELTA: usize = 100;
const SIGNIFICANT_RATIO: f64 = 0.10;

#[derive(Default)]
pub struct ChangeDetector {
    last_snapshot: Option<ContextSnapshot>,
}

impl ChangeDetector {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `new` differs meaningfully from the last snapshot
    /// seen (or if this is the first snapshot). Always records `new` as the
    /// baseline for the next call, whether or not it counted as a change.
    pub fn has_changed(&mut self, new: &ContextSnapshot) -> bool {
        let changed = match &self.last_snapshot {
            None => true,
            Some(prev) => {
                prev.hwnd != new.hwnd
                    || prev.window_title != new.window_title
                    || prev.url != new.url
                    || prev.document_path != new.document_path
                    || prev.selected_text != new.selected_text
                    || text_diff_significant(prev.visible_text.as_ref(), new.visible_text.as_ref())
            }
        };
        self.last_snapshot = Some(new.clone());
        changed
    }
}

fn text_diff_significant(prev: Option<&String>, new: Option<&String>) -> bool {
    match (prev, new) {
        (None, None) => false,
        (None, Some(_)) | (Some(_), None) => true,
        (Some(p), Some(n)) if p == n => false,
        (Some(p), Some(n)) => {
            let delta = p.len().abs_diff(n.len());
            let max_len = p.len().max(n.len()).max(1);
            #[allow(clippy::cast_precision_loss)]
            let ratio = delta as f64 / max_len as f64;
            delta > SIGNIFICANT_CHAR_DELTA || ratio > SIGNIFICANT_RATIO
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::CaptureMethod;
    use uuid::Uuid;

    use super::*;

    fn snapshot(hwnd: i64, title: &str, url: Option<&str>, text: Option<&str>) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: title.to_string(),
            process_name: "chrome.exe".to_string(),
            process_id: 1,
            hwnd: Some(hwnd),
            url: url.map(str::to_string),
            document_path: None,
            visible_text: text.map(str::to_string),
            selected_text: None,
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    #[test]
    fn first_snapshot_is_always_a_change() {
        let mut cd = ChangeDetector::new();
        assert!(cd.has_changed(&snapshot(1, "Tab A", None, None)));
    }

    #[test]
    fn identical_snapshot_is_not_a_change() {
        let mut cd = ChangeDetector::new();
        let s = snapshot(1, "Tab A", Some("https://a.com"), Some("hello"));
        assert!(cd.has_changed(&s));
        assert!(!cd.has_changed(&s));
    }

    #[test]
    fn hwnd_switch_is_a_change() {
        let mut cd = ChangeDetector::new();
        cd.has_changed(&snapshot(1, "Tab A", None, None));
        assert!(cd.has_changed(&snapshot(2, "Tab A", None, None)));
    }

    #[test]
    fn url_change_is_a_change() {
        let mut cd = ChangeDetector::new();
        cd.has_changed(&snapshot(1, "Tab A", Some("https://a.com"), None));
        assert!(cd.has_changed(&snapshot(1, "Tab A", Some("https://b.com"), None)));
    }

    #[test]
    fn small_text_tweak_is_not_a_change() {
        let mut cd = ChangeDetector::new();
        cd.has_changed(&snapshot(1, "Doc", None, Some(&"a".repeat(1000))));
        let mut tweaked = "a".repeat(1000);
        tweaked.push_str("bb"); // 2 chars added, well under both thresholds
        assert!(!cd.has_changed(&snapshot(1, "Doc", None, Some(&tweaked))));
    }

    #[test]
    fn large_length_delta_is_a_change() {
        let mut cd = ChangeDetector::new();
        cd.has_changed(&snapshot(1, "Doc", None, Some(&"a".repeat(1000))));
        assert!(cd.has_changed(&snapshot(1, "Doc", None, Some(&"a".repeat(1500)))));
    }
}
