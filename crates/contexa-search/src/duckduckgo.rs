//! `DuckDuckGoAdapter` — `docs/09_Search_Engine.md` §5.4, default provider
//! per ADR-0011 (zero-config, no API key).

use async_trait::async_trait;
use scraper::{Html, Selector};

use contexa_core::{ContexaError, Result};

use crate::adapter::SearchAdapter;
use crate::types::{SearchResult, WebSearchOptions};

const ENDPOINT: &str = "https://html.duckduckgo.com/html/";
const DEFAULT_MAX_RESULTS: usize = 5;

pub struct DuckDuckGoAdapter {
    client: reqwest::Client,
}

impl DuckDuckGoAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DuckDuckGoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchAdapter for DuckDuckGoAdapter {
    async fn search(&self, query: &str, opts: WebSearchOptions) -> Result<Vec<SearchResult>> {
        let response = self
            .client
            .get(ENDPOINT)
            .query(&[("q", query)])
            // The lite HTML endpoint serves a bare/degraded page to
            // unrecognized clients without a UA header.
            .header("User-Agent", "Mozilla/5.0 (compatible; ContexaBot/1.0)")
            .send()
            .await
            .map_err(ddg_err)?;

        if !response.status().is_success() {
            return Err(ContexaError::Conversion(format!(
                "DuckDuckGo HTTP {}",
                response.status()
            )));
        }

        let body = response.text().await.map_err(ddg_err)?;
        Ok(parse_results(&body, opts.max_results))
    }

    fn provider_name(&self) -> &'static str {
        "duckduckgo"
    }

    fn max_results(&self) -> usize {
        DEFAULT_MAX_RESULTS
    }
}

/// Selectors validated against `DuckDuckGo`'s HTML/lite endpoint markup as of
/// 2026-07 (see `examples/search_smoke.rs`). ADR-0011 explicitly flags
/// scraping breakage as a risk — degrade to an empty `Vec`, never
/// panic/error, if the selectors stop matching or nothing parses.
fn parse_results(html: &str, max_results: usize) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let (Ok(title_sel), Ok(snippet_sel)) = (
        Selector::parse("a.result__a"),
        Selector::parse("a.result__snippet"),
    ) else {
        return Vec::new();
    };

    let titles: Vec<_> = document.select(&title_sel).collect();
    let snippets: Vec<_> = document.select(&snippet_sel).collect();

    titles
        .into_iter()
        .zip(snippets)
        .take(max_results)
        .filter_map(|(title_el, snippet_el)| {
            let href = title_el.value().attr("href")?;
            let url = resolve_url(href);
            let title: String = title_el.text().collect::<String>().trim().to_string();
            let snippet: String = snippet_el.text().collect::<String>().trim().to_string();
            if title.is_empty() || url.is_empty() {
                return None;
            }
            Some(SearchResult {
                title,
                snippet,
                url,
                published_date: None,
                // Filled in by rank position once results reach the engine
                // (see `engine.rs`) — no per-result signal from the provider.
                relevance_score: 0.0,
            })
        })
        .collect()
}

/// `DuckDuckGo`'s HTML endpoint wraps result links in a redirect
/// (`//duckduckgo.com/l/?uddg=<percent-encoded-real-url>&...`) — decode to
/// the real destination so citations point somewhere useful.
fn resolve_url(href: &str) -> String {
    if let Some(idx) = href.find("uddg=") {
        let after = &href[idx + "uddg=".len()..];
        let encoded = after.split('&').next().unwrap_or(after);
        if let Ok(decoded) = urlencoding::decode(encoded) {
            return decoded.into_owned();
        }
    }
    if let Some(stripped) = href.strip_prefix("//") {
        return format!("https://{stripped}");
    }
    href.to_string()
}

// reqwest::Error is cheap to take by value (map_err's closure arrives owned
// regardless) — same rationale as contexa-llm's llm_err.
#[allow(clippy::needless_pass_by_value)]
fn ddg_err(e: reqwest::Error) -> ContexaError {
    ContexaError::Conversion(format!("DuckDuckGo request failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_redirect_link() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fdocs.rs%2Ftokio&rut=abc123";
        assert_eq!(resolve_url(href), "https://docs.rs/tokio");
    }

    #[test]
    fn resolves_a_protocol_relative_link() {
        assert_eq!(resolve_url("//example.com/page"), "https://example.com/page");
    }

    #[test]
    fn passes_through_an_absolute_link() {
        assert_eq!(resolve_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn parses_real_lite_html_markup_shape() {
        // Minimal excerpt matching DuckDuckGo's documented result markup —
        // full page structure is validated live in `examples/search_smoke.rs`.
        let html = r#"
            <div class="result">
                <h2 class="result__title">
                    <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdocs.rs%2Ftokio">Tokio docs</a>
                </h2>
                <a class="result__snippet">Async runtime for Rust.</a>
            </div>
        "#;
        let results = parse_results(html, 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Tokio docs");
        assert_eq!(results[0].url, "https://docs.rs/tokio");
        assert_eq!(results[0].snippet, "Async runtime for Rust.");
    }

    #[test]
    fn unrecognized_markup_degrades_to_empty_not_error() {
        assert_eq!(parse_results("<html><body>nothing here</body></html>", 5).len(), 0);
    }
}
