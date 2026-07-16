//! Selection Tracker — `docs/06_Context_Engine.md` §5.5. Tries UIA
//! `TextPattern` selection first, falls back to clipboard content.
//!
//! Deviates from docs' `poll(&mut self, hwnd: isize) -> Option<String>`
//! ("return selection if changed") in one way: this always returns the
//! *current* selection state (or `None`), not just on change. Gating on
//! "did it change" is already `ChangeDetector`'s job (it compares
//! `selected_text` between snapshots) — duplicating that here would just
//! be two places that can disagree about what "changed" means.

use contexa_vision::{clipboard, with_sta_com, UiaExtractor};

/// Abstraction over "how do we get the current selection" so
/// `ContexaContextEngine` can swap in a no-op for contexts that must not
/// touch real UIA/clipboard state (see `NoSelectionSource`).
pub trait SelectionSource: Send {
    fn poll(&mut self) -> Option<String>;
}

/// Always returns `None` — the default in `ContexaContextEngine::new()`.
/// Real UIA (COM, apartment-threaded, expects a message pump) and clipboard
/// access are not safe to exercise from many parallel threads without one,
/// which is exactly how `cargo test` runs; wiring the real `SelectionTracker`
/// in unconditionally crashed the test binary with
/// `STATUS_HEAP_CORRUPTION` during development. Same reason
/// `contexa-vision` keeps all live UIA/COM code out of its unit tests.
pub struct NoSelectionSource;

impl SelectionSource for NoSelectionSource {
    fn poll(&mut self) -> Option<String> {
        None
    }
}

pub struct SelectionTracker {
    last_clipboard_seq: u32,
    last_clipboard_text: Option<String>,
}

impl Default for SelectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_clipboard_seq: 0,
            last_clipboard_text: None,
        }
    }

    fn poll_inner(&mut self) -> Option<String> {
        let uia_selection = with_sta_com(|| {
            UiaExtractor::new()
                .ok()
                .and_then(|extractor| extractor.get_selected_text())
        })
        .ok()
        .flatten();

        if uia_selection.is_some() {
            return uia_selection;
        }
        self.clipboard_selection()
    }

    fn clipboard_selection(&mut self) -> Option<String> {
        let seq = clipboard::sequence_number();
        if self.should_reread(seq) {
            self.last_clipboard_seq = seq;
            self.last_clipboard_text = clipboard::read_text();
        }
        self.last_clipboard_text.clone()
    }

    /// Pulled out for testing: the clipboard is cheap to poll for its
    /// sequence number but reading its content is comparatively expensive,
    /// so only re-read when the sequence number actually moved.
    fn should_reread(&self, current_seq: u32) -> bool {
        current_seq != self.last_clipboard_seq
    }
}

impl SelectionSource for SelectionTracker {
    fn poll(&mut self) -> Option<String> {
        self.poll_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rereads_only_when_sequence_number_changes() {
        let mut tracker = SelectionTracker::new();
        assert!(tracker.should_reread(1));
        tracker.last_clipboard_seq = 1;
        assert!(!tracker.should_reread(1));
        assert!(tracker.should_reread(2));
    }
}
