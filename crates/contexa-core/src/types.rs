//! Shared context types — see `docs/02_System_Architecture.md` §8.1 and
//! `docs/04_Database_Design.md` §5.1.
//!
//! Fields follow the `context_snapshots` table row shape (flat, not the
//! `WindowInfo`/`ApplicationInfo` sketch in docs/02, which isn't specified
//! further anywhere) so this type round-trips through the DB layer directly.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMethod {
    Uia,
    Ocr,
    Hybrid,
}

impl CaptureMethod {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureMethod::Uia => "uia",
            CaptureMethod::Ocr => "ocr",
            CaptureMethod::Hybrid => "hybrid",
        }
    }
}

impl std::str::FromStr for CaptureMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uia" => Ok(CaptureMethod::Uia),
            "ocr" => Ok(CaptureMethod::Ocr),
            "hybrid" => Ok(CaptureMethod::Hybrid),
            other => Err(format!("unknown capture_method: {other}")),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextSnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub window_title: String,
    pub process_name: String,
    pub process_id: i64,
    pub hwnd: Option<i64>,
    pub url: Option<String>,
    pub document_path: Option<String>,
    pub visible_text: Option<String>,
    pub selected_text: Option<String>,
    pub metadata: HashMap<String, String>,
    pub language: Option<String>,
    pub capture_method: CaptureMethod,
}

// docs/08_AI_Orchestrator.md §9, §5.1, §8 — shared by `contexa-prompt` and
// `contexa-orchestrator`. Living here (rather than in `contexa-orchestrator`,
// which is where docs/08 defines them) avoids a dependency cycle: Orchestrator
// depends on Prompt Builder, so Prompt Builder can't depend back on
// Orchestrator for these types. Same reasoning that put `ContextSnapshot` here.

#[derive(Debug, Clone)]
pub struct UserRequest {
    pub id: Uuid,
    pub action: RequestAction,
    pub query: Option<String>,
    pub context_override: Option<ContextSnapshot>,
    pub preferences: RequestPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestAction {
    Chat,
    Explain,
    Summarize,
    Translate { target_lang: String },
    Search,
    Recall,
}

#[derive(Debug, Clone, Copy)]
pub struct RequestPreferences {
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub force_search: bool,
    pub force_ocr: bool,
}

impl Default for RequestPreferences {
    fn default() -> Self {
        Self {
            stream: true,
            max_tokens: None,
            temperature: None,
            force_search: false,
            force_ocr: false,
        }
    }
}

// Shape is docs/08_AI_Orchestrator.md §8's `ExecutionPlan` verbatim — each
// field is an independent yes/no decision the Decision Engine makes, not
// exclusive states an enum would fit.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub need_context: bool,
    pub need_ocr: bool,
    pub need_memory: bool,
    pub need_timeline: bool,
    pub need_search: bool,
    pub need_mcp: bool,
}

#[derive(Debug, Clone)]
pub struct RequestHandle {
    pub id: String,
    pub status: RequestStatus,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestStatus {
    Planning,
    Gathering,
    Generating,
    Complete,
    Failed(String),
    Cancelled,
}
