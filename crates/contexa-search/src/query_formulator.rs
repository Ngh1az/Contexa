//! `QueryFormulator` — `docs/09_Search_Engine.md` §5.2.

use contexa_core::ContextSnapshot;

pub struct QueryFormulator;

impl QueryFormulator {
    #[must_use]
    pub fn formulate(&self, user_query: &str, context: &ContextSnapshot) -> String {
        let mut parts = vec![user_query.to_string()];

        if let Some(url) = &context.url {
            if let Some(domain) = extract_domain(url) {
                parts.push(format!("site:{domain}"));
            }
        }

        if let Some(doc) = &context.document_path {
            if let Some(ext) = path_extension(doc) {
                parts.push(format!("filetype:{ext}"));
            }
        }

        parts.join(" ")
    }
}

fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let domain = without_scheme.split('/').next()?;
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

fn path_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::CaptureMethod;
    use uuid::Uuid;

    use super::*;

    fn bare_context() -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: "t".to_string(),
            process_name: "p".to_string(),
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
    fn plain_query_with_no_hints() {
        let formulated = QueryFormulator.formulate("rust ownership", &bare_context());
        assert_eq!(formulated, "rust ownership");
    }

    #[test]
    fn adds_site_hint_from_url() {
        let mut context = bare_context();
        context.url = Some("https://docs.rs/tokio/latest/tokio/".to_string());
        let formulated = QueryFormulator.formulate("spawn_blocking", &context);
        assert_eq!(formulated, "spawn_blocking site:docs.rs");
    }

    #[test]
    fn adds_filetype_hint_from_document_path() {
        let mut context = bare_context();
        context.document_path = Some("D:\\Contexa\\src\\main.rs".to_string());
        let formulated = QueryFormulator.formulate("borrow checker", &context);
        assert_eq!(formulated, "borrow checker filetype:rs");
    }
}
