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
