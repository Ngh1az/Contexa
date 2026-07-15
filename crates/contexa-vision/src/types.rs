//! Vision Engine data structures — `docs/05_Vision_Engine.md` §8.

use chrono::{DateTime, Utc};
use contexa_core::CaptureMethod;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>, // BGRA
    pub width: u32,
    pub height: u32,
    pub timestamp: DateTime<Utc>,
}

/// Returned by `get_window_info()` and emitted on focus change (docs/05 §5.1's
/// `WindowFocusEvent` is the same shape — one struct, not two).
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub process_id: u32,
    pub process_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UiaResult {
    pub text: String,
    pub element_count: u32,
    pub tree_depth: u32,
    pub confidence: f32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub regions: Vec<Region>,
    pub confidence: f32,
    pub cached: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct VisionResult {
    pub hwnd: isize,
    pub window_title: String,
    pub process_name: String,
    pub process_id: u32,
    pub frame_hash: [u64; 4],
    pub changed_regions: Vec<Region>,
    pub uia_result: Option<UiaResult>,
    pub ocr_result: Option<OcrResult>,
    pub capture_method: Option<CaptureMethod>,
    pub timestamp: DateTime<Utc>,
}
