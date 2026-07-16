//! Chrome/Edge enricher — `docs/06_Context_Engine.md` §5.2, `docs/18_Plugin_System.md`
//! §6. Both are Chromium and share the same address-bar `AutomationId`
//! (`"addressEditBox"`), so one implementation covers both, configured via
//! `ChromiumEnricher::chrome()` / `::edge()` rather than two near-duplicate
//! trait impls.
//!
//! SP-01 (`benchmarks/BASELINE.md`) validated Chrome as UIA-capable, unlike
//! VS Code — see `enrichers::vscode` for why that one takes a different
//! (title-parsing) approach.

use contexa_core::{ContextSnapshot, Result};
use contexa_vision::{with_sta_com, UiaExtractor};

use crate::enricher::{ContextEnricher, PluginInfo};

const ADDRESS_BAR_AUTOMATION_ID: &str = "addressEditBox";
const PRIORITY: u32 = 100; // docs/18 §6

pub struct ChromiumEnricher {
    process_name: &'static str,
    browser_tag: &'static str,
    plugin_id: &'static str,
    plugin_name: &'static str,
}

impl ChromiumEnricher {
    #[must_use]
    pub fn chrome() -> Self {
        Self {
            process_name: "chrome.exe",
            browser_tag: "chrome",
            plugin_id: "enricher.chrome",
            plugin_name: "Chrome Enricher",
        }
    }

    #[must_use]
    pub fn edge() -> Self {
        Self {
            process_name: "msedge.exe",
            browser_tag: "edge",
            plugin_id: "enricher.edge",
            plugin_name: "Edge Enricher",
        }
    }
}

impl ContextEnricher for ChromiumEnricher {
    fn matches(&self, process_name: &str) -> bool {
        process_name.eq_ignore_ascii_case(self.process_name)
    }

    fn enrich(&self, snapshot: &mut ContextSnapshot) -> Result<()> {
        let Some(hwnd) = snapshot.hwnd.and_then(|h| isize::try_from(h).ok()) else {
            return Ok(()); // no window handle on the snapshot — nothing to query
        };
        let url = with_sta_com(|| {
            UiaExtractor::new()
                .ok()
                .and_then(|extractor| extractor.find_by_automation_id(hwnd, ADDRESS_BAR_AUTOMATION_ID))
        })?;
        if let Some(url) = url {
            snapshot.url = Some(url);
        }
        snapshot
            .metadata
            .insert("browser".to_string(), self.browser_tag.to_string());
        Ok(())
    }

    fn priority(&self) -> u32 {
        PRIORITY
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.plugin_id.to_string(),
            name: self.plugin_name.to_string(),
            version: "0.1.0".to_string(),
            author: "Contexa Team".to_string(),
            description: format!("Extracts URL from the {} address bar", self.process_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_matches_process_name_case_insensitively() {
        let e = ChromiumEnricher::chrome();
        assert!(e.matches("chrome.exe"));
        assert!(e.matches("CHROME.EXE"));
        assert!(!e.matches("msedge.exe"));
    }

    #[test]
    fn edge_matches_process_name_case_insensitively() {
        let e = ChromiumEnricher::edge();
        assert!(e.matches("msedge.exe"));
        assert!(!e.matches("chrome.exe"));
    }

    #[test]
    fn plugin_ids_are_distinct() {
        assert_eq!(ChromiumEnricher::chrome().info().id, "enricher.chrome");
        assert_eq!(ChromiumEnricher::edge().info().id, "enricher.edge");
    }
}
