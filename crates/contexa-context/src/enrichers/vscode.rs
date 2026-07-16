//! VS Code enricher — title parsing, not live UIA. `benchmarks/BASELINE.md`
//! recorded VS Code/Monaco as UIA-opaque ("title enricher (v1.0) + LSP
//! extension (v1.1, docs/27)"); real IDE context (symbols, git branch,
//! absolute workspace path) is deferred to that LSP extension per
//! `docs/14_Development_Roadmap.md` §10.2. For v1.0 this enricher only
//! extracts what's visible in the window title: the active file name, a
//! language guess from its extension, and the workspace/folder name.

use contexa_core::{ContextSnapshot, Result};

use crate::enricher::{ContextEnricher, PluginInfo};

const PROCESS_NAME: &str = "Code.exe";
const PRIORITY: u32 = 100; // docs/18 §6
const TITLE_SUFFIXES: [&str; 2] = [" - Visual Studio Code", " — Visual Studio Code"];
const SEPARATORS: [&str; 2] = [" - ", " — "];
const UNSAVED_MARKER: char = '\u{25cf}'; // "●"

pub struct VsCodeEnricher;

impl ContextEnricher for VsCodeEnricher {
    fn matches(&self, process_name: &str) -> bool {
        process_name.eq_ignore_ascii_case(PROCESS_NAME)
    }

    fn enrich(&self, snapshot: &mut ContextSnapshot) -> Result<()> {
        let Some(parsed) = parse_title(&snapshot.window_title) else {
            return Ok(());
        };
        if let Some(lang) = language_from_extension(&parsed.file_name) {
            snapshot
                .metadata
                .insert("language_hint".to_string(), lang.to_string());
        }
        if let Some(workspace) = parsed.workspace {
            snapshot.metadata.insert("workspace".to_string(), workspace);
        }
        snapshot.document_path = Some(parsed.file_name);
        Ok(())
    }

    fn priority(&self) -> u32 {
        PRIORITY
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "enricher.vscode".to_string(),
            name: "VS Code Enricher".to_string(),
            version: "0.1.0".to_string(),
            author: "Contexa Team".to_string(),
            description: "Extracts file name and workspace from the VS Code window title"
                .to_string(),
        }
    }
}

struct ParsedTitle {
    file_name: String,
    workspace: Option<String>,
}

fn parse_title(title: &str) -> Option<ParsedTitle> {
    let body = TITLE_SUFFIXES
        .iter()
        .find_map(|suffix| title.strip_suffix(suffix))?;

    let (file_part, workspace) = match SEPARATORS.iter().find(|sep| body.contains(*sep)) {
        Some(sep) => {
            let mut split = body.splitn(2, sep);
            let file_part = split.next().unwrap_or(body);
            (file_part, split.next().map(str::to_string))
        }
        None => (body, None),
    };

    let file_name = file_part.trim_start_matches(UNSAVED_MARKER).trim().to_string();
    if file_name.is_empty() {
        return None;
    }
    Some(ParsedTitle {
        file_name,
        workspace,
    })
}

fn language_from_extension(file_name: &str) -> Option<&'static str> {
    let ext = file_name.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "cc" | "cxx" | "h" | "hpp" => "cpp",
        "cs" => "csharp",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
        "sql" => "sql",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use contexa_core::CaptureMethod;
    use uuid::Uuid;

    use super::*;

    fn snapshot_with_title(title: &str) -> ContextSnapshot {
        ContextSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            window_title: title.to_string(),
            process_name: "Code.exe".to_string(),
            process_id: 1,
            hwnd: Some(1),
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
    fn matches_code_exe_case_insensitively() {
        let e = VsCodeEnricher;
        assert!(e.matches("Code.exe"));
        assert!(e.matches("code.exe"));
        assert!(!e.matches("chrome.exe"));
    }

    #[test]
    fn parses_file_and_workspace_from_title() {
        let Some(parsed) = parse_title("main.rs - Contexa - Visual Studio Code") else {
            panic!("expected a parsed title");
        };
        assert_eq!(parsed.file_name, "main.rs");
        assert_eq!(parsed.workspace.as_deref(), Some("Contexa"));
    }

    #[test]
    fn parses_em_dash_title_format() {
        let Some(parsed) = parse_title("main.rs — Contexa — Visual Studio Code") else {
            panic!("expected a parsed title");
        };
        assert_eq!(parsed.file_name, "main.rs");
        assert_eq!(parsed.workspace.as_deref(), Some("Contexa"));
    }

    #[test]
    fn strips_unsaved_marker() {
        let Some(parsed) = parse_title("● main.rs - Contexa - Visual Studio Code") else {
            panic!("expected a parsed title");
        };
        assert_eq!(parsed.file_name, "main.rs");
    }

    #[test]
    fn handles_title_without_workspace() {
        let Some(parsed) = parse_title("main.rs - Visual Studio Code") else {
            panic!("expected a parsed title");
        };
        assert_eq!(parsed.file_name, "main.rs");
        assert_eq!(parsed.workspace, None);
    }

    #[test]
    fn non_vscode_title_is_not_parsed() {
        assert!(parse_title("Inbox - Outlook").is_none());
    }

    #[test]
    fn enrich_sets_document_path_and_language_hint() {
        let e = VsCodeEnricher;
        let mut snapshot = snapshot_with_title("main.rs - Contexa - Visual Studio Code");
        assert!(e.enrich(&mut snapshot).is_ok());
        assert_eq!(snapshot.document_path.as_deref(), Some("main.rs"));
        assert_eq!(
            snapshot.metadata.get("language_hint").map(String::as_str),
            Some("rust")
        );
        assert_eq!(
            snapshot.metadata.get("workspace").map(String::as_str),
            Some("Contexa")
        );
    }

    #[test]
    fn unrecognized_extension_has_no_language_hint() {
        assert_eq!(language_from_extension("README"), None);
        assert_eq!(language_from_extension("script.rs"), Some("rust"));
    }
}
