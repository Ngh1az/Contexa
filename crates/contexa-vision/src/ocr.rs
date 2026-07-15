//! OCR Engine — `docs/05_Vision_Engine.md` §5.6. **Not implemented**:
//! `spikes/SP-03-ocr-fallback` was never run (`benchmarks/BASELINE.md`: "not
//! run — non-blocking"). Building `Windows.Media.Ocr` integration without
//! that spike first would skip this project's own spike-first gate
//! (`docs/22_Technical_Spike_Plan.md`). This returns an honest error instead
//! of a fake result.

use contexa_core::{ContexaError, Result};

use crate::types::{Frame, OcrResult, Region};

#[derive(Default)]
pub struct OcrEngine;

impl OcrEngine {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// # Errors
    /// Always returns `ContexaError::CaptureFailed` — see module docs.
    pub fn ocr_region(&self, _frame: &Frame, _region: &Region) -> Result<OcrResult> {
        Err(ContexaError::CaptureFailed {
            reason: "OCR not implemented — SP-03 not run, see docs/22".to_string(),
        })
    }
}
