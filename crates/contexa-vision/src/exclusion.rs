//! Exclusion Filter — `docs/05_Vision_Engine.md` §5, run before any capture.

use crate::types::WindowInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionKind {
    App,
    Url,
    WindowTitle,
}

#[derive(Debug, Clone)]
pub struct ExclusionRule {
    pub kind: ExclusionKind,
    pub pattern: String,
}

/// URL-kind rules are accepted but never match here — `WindowInfo` has no URL;
/// enforcing those is the Context Engine's job once it knows the page URL.
pub struct ExclusionFilter {
    rules: Vec<ExclusionRule>,
}

impl ExclusionFilter {
    #[must_use]
    pub fn new(rules: Vec<ExclusionRule>) -> Self {
        Self { rules }
    }

    /// Privacy-sensitive app exclusions enabled by default (`docs/16_Security_Privacy.md`
    /// §6.2) — password managers and financial/healthcare apps are never
    /// captured unless the user removes them in Settings (Phase 3, not built
    /// yet). The URL patterns from the same list aren't included here — see
    /// this struct's doc comment above.
    #[must_use]
    pub fn default_rules() -> Vec<ExclusionRule> {
        const APPS: &[&str] = &[
            "1password.exe",
            "bitwarden.exe",
            "lastpass.exe",
            "keepass.exe",
            "mint.exe",
            "quicken.exe",
            "teladoc.exe",
            "mychart.exe",
        ];
        APPS.iter()
            .map(|app| ExclusionRule {
                kind: ExclusionKind::App,
                pattern: (*app).to_string(),
            })
            .collect()
    }

    #[must_use]
    pub fn is_excluded(&self, window: &WindowInfo) -> bool {
        self.rules.iter().any(|rule| match rule.kind {
            ExclusionKind::App => matches_pattern(&rule.pattern, &window.process_name),
            ExclusionKind::WindowTitle => matches_pattern(&rule.pattern, &window.title),
            ExclusionKind::Url => false,
        })
    }
}

/// Case-insensitive match; a `*` anywhere in the pattern means "contains" the
/// surrounding literal text instead of requiring an exact match.
fn matches_pattern(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let value = value.to_lowercase();
    if let Some(needle) = pattern.strip_prefix('*').and_then(|p| p.strip_suffix('*')) {
        return value.contains(needle);
    }
    if let Some(needle) = pattern.strip_prefix('*') {
        return value.ends_with(needle);
    }
    if let Some(needle) = pattern.strip_suffix('*') {
        return value.starts_with(needle);
    }
    value == pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn window(process_name: &str, title: &str) -> WindowInfo {
        WindowInfo {
            hwnd: 1,
            title: title.to_string(),
            process_id: 100,
            process_name: process_name.to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn exact_app_match_excludes() {
        let filter = ExclusionFilter::new(vec![ExclusionRule {
            kind: ExclusionKind::App,
            pattern: "keepass.exe".to_string(),
        }]);
        assert!(filter.is_excluded(&window("KeePass.exe", "KeePass")));
    }

    #[test]
    fn wildcard_title_match_excludes() {
        let filter = ExclusionFilter::new(vec![ExclusionRule {
            kind: ExclusionKind::WindowTitle,
            pattern: "*private*".to_string(),
        }]);
        assert!(filter.is_excluded(&window("chrome.exe", "My Private Browsing Session")));
    }

    #[test]
    fn no_match_is_not_excluded() {
        let filter = ExclusionFilter::new(vec![ExclusionRule {
            kind: ExclusionKind::App,
            pattern: "keepass.exe".to_string(),
        }]);
        assert!(!filter.is_excluded(&window("chrome.exe", "GitHub")));
    }

    #[test]
    fn default_rules_exclude_known_password_manager() {
        let filter = ExclusionFilter::new(ExclusionFilter::default_rules());
        assert!(filter.is_excluded(&window("KeePass.exe", "KeePass")));
        assert!(filter.is_excluded(&window("1Password.exe", "1Password")));
    }

    #[test]
    fn default_rules_do_not_exclude_unrelated_app() {
        let filter = ExclusionFilter::new(ExclusionFilter::default_rules());
        assert!(!filter.is_excluded(&window("chrome.exe", "GitHub")));
    }

    #[test]
    fn url_rules_never_match_here() {
        let filter = ExclusionFilter::new(vec![ExclusionRule {
            kind: ExclusionKind::Url,
            pattern: "*bank.com*".to_string(),
        }]);
        assert!(!filter.is_excluded(&window("chrome.exe", "bank.com — Login")));
    }
}
