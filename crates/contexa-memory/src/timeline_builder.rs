//! `TimelineBuilder` — `docs/07_Memory_Engine.md` §6.2: turns context changes
//! into human-readable timeline events, debouncing same-app/same-window
//! repeats.

use contexa_core::ContextSnapshot;
use contexa_db::{EventType, TimelineEvent};
use uuid::Uuid;

#[derive(Default)]
pub struct TimelineBuilder {
    last_event: Option<TimelineEvent>,
}

impl TimelineBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `None` when `snapshot` is the same app+window as the last
    /// event (debounced — not a new event).
    ///
    /// Spec's pseudocode (docs/07 §6.2) also back-fills `duration_ms` onto
    /// the *previous* event when a new one opens, but `TimelineRepository`
    /// only supports `insert_event`, not an update — so that duration is
    /// tracked here in memory (for `TimelineBuilder`'s own bookkeeping) but
    /// isn't retroactively persisted. Only the newly opened event is
    /// returned for the caller to insert.
    pub fn process_context_change(&mut self, snapshot: &ContextSnapshot) -> Option<TimelineEvent> {
        if let Some(prev) = &self.last_event {
            if prev.application == snapshot.process_name && prev.window_title == snapshot.window_title
            {
                return None;
            }
        }

        let event_type = self.classify_event(snapshot);
        let event = TimelineEvent {
            id: Uuid::new_v4(),
            timestamp: snapshot.timestamp,
            event_type,
            summary: generate_summary(snapshot),
            application: snapshot.process_name.clone(),
            window_title: snapshot.window_title.clone(),
            duration_ms: None,
            context_id: Some(snapshot.id),
        };

        if let Some(prev) = &mut self.last_event {
            let duration = (event.timestamp - prev.timestamp).num_milliseconds();
            prev.duration_ms = Some(duration.max(0));
        }

        self.last_event = Some(event.clone());
        Some(event)
    }

    fn classify_event(&self, snapshot: &ContextSnapshot) -> EventType {
        match &self.last_event {
            Some(prev) if prev.application != snapshot.process_name => EventType::AppSwitch,
            _ => EventType::ContextChange,
        }
    }
}

fn generate_summary(snapshot: &ContextSnapshot) -> String {
    match (&snapshot.url, &snapshot.document_path) {
        (Some(url), _) => format!("Browsing: {}", truncate(url, 80)),
        (_, Some(path)) => format!("Editing: {}", file_name(path)),
        _ => format!(
            "Using {}: {}",
            snapshot.process_name,
            truncate(&snapshot.window_title, 60)
        ),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::CaptureMethod;

    use super::*;

    fn snapshot(process_name: &str, window_title: &str) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: window_title.to_string(),
            process_name: process_name.to_string(),
            process_id: 1,
            hwnd: None,
            url: None,
            document_path: None,
            visible_text: None,
            selected_text: None,
            metadata: HashMap::new(),
            language: None,
            capture_method: CaptureMethod::Uia,
        }
    }

    #[test]
    fn first_snapshot_produces_a_context_change_event() {
        let mut tb = TimelineBuilder::new();
        let event = tb
            .process_context_change(&snapshot("Code.exe", "main.rs"))
            .expect("first snapshot should produce an event");
        assert_eq!(event.event_type, EventType::ContextChange);
    }

    #[test]
    fn same_app_and_window_is_debounced() {
        let mut tb = TimelineBuilder::new();
        tb.process_context_change(&snapshot("Code.exe", "main.rs"));
        let second = tb.process_context_change(&snapshot("Code.exe", "main.rs"));
        assert!(second.is_none());
    }

    #[test]
    fn different_app_produces_an_app_switch_event() {
        let mut tb = TimelineBuilder::new();
        tb.process_context_change(&snapshot("Code.exe", "main.rs"));
        let event = tb
            .process_context_change(&snapshot("chrome.exe", "GitHub"))
            .expect("app switch should produce an event");
        assert_eq!(event.event_type, EventType::AppSwitch);
    }

    #[test]
    fn same_app_different_window_produces_a_context_change_event() {
        let mut tb = TimelineBuilder::new();
        tb.process_context_change(&snapshot("Code.exe", "main.rs"));
        let event = tb
            .process_context_change(&snapshot("Code.exe", "lib.rs"))
            .expect("window change should produce an event");
        assert_eq!(event.event_type, EventType::ContextChange);
    }

    #[test]
    fn summary_prefers_url_then_document_path_then_window_title() {
        let mut with_url = snapshot("chrome.exe", "GitHub");
        with_url.url = Some("https://github.com/anthropics/claude-code".to_string());
        assert!(generate_summary(&with_url).starts_with("Browsing:"));

        let mut with_doc = snapshot("Code.exe", "main.rs — VS Code");
        with_doc.document_path = Some("D:\\Contexa\\src\\main.rs".to_string());
        assert_eq!(generate_summary(&with_doc), "Editing: main.rs");

        let plain = snapshot("notepad.exe", "Untitled");
        assert_eq!(generate_summary(&plain), "Using notepad.exe: Untitled");
    }
}
