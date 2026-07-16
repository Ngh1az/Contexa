//! OCR Engine — `docs/05_Vision_Engine.md` §5.6. Implemented after
//! `spikes/SP-03-ocr-fallback` (Partial Pass — see its `report.md`):
//! `Windows.Media.Ocr` against a full-window WGC capture recognized text in
//! 94-106ms p50/p95, well under the 500ms target, reusing the exact
//! frame -> `SoftwareBitmap` conversion validated there (including the
//! force-alpha-255 fix — WGC's alpha channel isn't guaranteed meaningful for
//! opaque windows, and Premultiplied-alpha with alpha=0 renders solid black).

use std::time::Instant;

use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine as WinOcrEngine;
use windows::Storage::Streams::DataWriter;

use contexa_core::{ContexaError, Result};

use crate::types::{Frame, OcrResult, Region};

#[derive(Default)]
pub struct OcrEngine;

impl OcrEngine {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Crops `frame` to `region`, runs `Windows.Media.Ocr` on it, and
    /// returns the recognized text. Must be called from a thread with COM
    /// initialized (ADR-0008 pattern — see `engine.rs`'s one-shot OCR
    /// thread); `Windows.Media.Ocr` is a WinRT/COM API.
    ///
    /// # Errors
    /// Returns an error if `region` is empty/out of bounds, no OCR language
    /// pack is installed, or recognition fails.
    pub fn ocr_region(&self, frame: &Frame, region: &Region) -> Result<OcrResult> {
        let started = Instant::now();
        let (data, width, height) = crop(frame, region);
        if width == 0 || height == 0 {
            return Err(ContexaError::CaptureFailed {
                reason: "OCR region is empty or out of frame bounds".to_string(),
            });
        }

        let bitmap = frame_to_software_bitmap(&data, width, height)?;
        let engine = WinOcrEngine::TryCreateFromUserProfileLanguages().map_err(win_err)?;
        let recognized = engine.RecognizeAsync(&bitmap).map_err(win_err)?.join().map_err(win_err)?;
        let text = recognized.Text().map_err(win_err)?.to_string_lossy();

        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(OcrResult {
            confidence: if text.is_empty() { 0.0 } else { 1.0 },
            text,
            regions: vec![*region],
            cached: false,
            duration_ms,
        })
    }
}

// windows::core::Error is a thin HRESULT wrapper — cheap to take by value,
// and map_err's closure argument arrives owned regardless (capture.rs's
// win_err has the identical shape/rationale).
#[allow(clippy::needless_pass_by_value)]
fn win_err(e: windows::core::Error) -> ContexaError {
    ContexaError::CaptureFailed {
        reason: e.to_string(),
    }
}

/// Copies out the `region` sub-rectangle of `frame`'s BGRA buffer,
/// clamping to the frame's actual bounds.
fn crop(frame: &Frame, region: &Region) -> (Vec<u8>, u32, u32) {
    let x = region.x.min(frame.width);
    let y = region.y.min(frame.height);
    let width = region.width.min(frame.width.saturating_sub(x));
    let height = region.height.min(frame.height.saturating_sub(y));
    if width == 0 || height == 0 {
        return (Vec::new(), 0, 0);
    }

    let row_bytes = width as usize * 4;
    let mut data = vec![0u8; row_bytes * height as usize];
    for row in 0..height as usize {
        let src_start = ((y as usize + row) * frame.width as usize + x as usize) * 4;
        let src = &frame.data[src_start..src_start + row_bytes];
        data[row * row_bytes..(row + 1) * row_bytes].copy_from_slice(src);
    }
    (data, width, height)
}

/// See module doc comment: forces full opacity so `SoftwareBitmap`'s
/// default (Premultiplied) alpha interpretation can't render this black.
fn frame_to_software_bitmap(bgra: &[u8], width: u32, height: u32) -> Result<SoftwareBitmap> {
    let mut data = bgra.to_vec();
    for pixel in data.chunks_exact_mut(4) {
        pixel[3] = 255;
    }

    let writer = DataWriter::new().map_err(win_err)?;
    writer.WriteBytes(&data).map_err(win_err)?;
    let buffer = writer.DetachBuffer().map_err(win_err)?;
    SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        i32::try_from(width).map_err(|e| ContexaError::Conversion(e.to_string()))?,
        i32::try_from(height).map_err(|e| ContexaError::Conversion(e.to_string()))?,
    )
    .map_err(win_err)
}
