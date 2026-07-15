//! Vision Engine — capture, UIA, OCR — see `docs/05_Vision_Engine.md`

mod capture;
mod differencer;
mod engine;
mod exclusion;
mod hash;
mod ocr;
mod region_hash;
mod scheduler;
mod types;
mod uia;
mod window_monitor;

pub use capture::FrameCapturer;
pub use differencer::FrameDifferencer;
pub use engine::{ContexaVisionEngine, VisionEngine};
pub use exclusion::{ExclusionFilter, ExclusionKind, ExclusionRule};
pub use ocr::OcrEngine;
pub use region_hash::RegionHashCache;
pub use scheduler::{AdaptiveScheduler, CaptureState};
pub use types::{Frame, OcrResult, Region, UiaResult, VisionResult, WindowInfo};
pub use uia::UiaExtractor;
pub use window_monitor::WindowMonitor;
