//! `VisionEngine` trait + `ContexaVisionEngine` — `docs/05_Vision_Engine.md`
//! §7, §9. Capture thread follows ADR-0008 Pattern A (single STA thread owns
//! UIA + WGC, validated zero-COM-error in `spikes/SP-08-com-threading`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, TrySendError};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED,
};

use contexa_core::{CaptureMethod, ContexaError, Result};

use crate::capture::FrameCapturer;
use crate::differencer::FrameDifferencer;
use crate::exclusion::{ExclusionFilter, ExclusionRule};
use crate::hash::ahash;
use crate::ocr::OcrEngine;
use crate::scheduler::{AdaptiveScheduler, CaptureState};
use crate::types::{Frame, OcrResult, Region, UiaResult, VisionResult, WindowInfo};
use crate::uia::UiaExtractor;
use crate::window_monitor::WindowMonitor;

const CHANNEL_CAPACITY: usize = 16; // docs/05 §9

pub trait VisionEngine: Send + Sync {
    /// # Errors
    /// Returns an error if the capture thread can't be spawned.
    fn start(&self) -> Result<()>;
    /// # Errors
    /// Returns an error if the capture thread panicked.
    fn stop(&self) -> Result<()>;
    /// # Errors
    /// Returns an error if there's no foreground window or capture fails.
    fn capture_active_window(&self) -> Result<VisionResult>;
    /// # Errors
    /// Returns an error if `hwnd` has no accessible UIA root element.
    fn extract_uia_text(&self, hwnd: isize) -> Result<UiaResult>;
    /// # Errors
    /// Always errors — OCR isn't implemented yet (SP-03 not run).
    fn ocr_region(&self, region: &Region) -> Result<OcrResult>;
    /// # Errors
    /// Returns an error if there's no foreground window.
    fn get_window_info(&self) -> Result<WindowInfo>;
    fn is_excluded(&self, hwnd: isize) -> bool;
}

pub struct ContexaVisionEngine {
    exclusion: Arc<ExclusionFilter>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    tx: Sender<VisionResult>,
}

impl ContexaVisionEngine {
    /// Exclusion rules are injected (docs/02 §5.2: `Vision --> Core` only, no
    /// `Vision --> Db`) — loading them from `contexa-db`'s `exclusions` table
    /// is the caller's job. Returns the engine plus the receiving end of its
    /// result channel; nothing consumes it yet (Context Engine's job, next).
    #[must_use]
    pub fn new(rules: Vec<ExclusionRule>) -> (Self, Receiver<VisionResult>) {
        let (tx, rx) = crossbeam_channel::bounded(CHANNEL_CAPACITY);
        let engine = Self {
            exclusion: Arc::new(ExclusionFilter::new(rules)),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
            tx,
        };
        (engine, rx)
    }
}

impl Drop for ContexaVisionEngine {
    fn drop(&mut self) {
        // Best-effort: unblocks the loop so the thread exits on its own;
        // Drop can't return Result, so we don't join here.
        self.running.store(false, Ordering::SeqCst);
    }
}

impl VisionEngine for ContexaVisionEngine {
    fn start(&self) -> Result<()> {
        let mut guard = self
            .thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_some() {
            return Ok(()); // already running
        }
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let exclusion = self.exclusion.clone();
        let tx = self.tx.clone();

        let handle = std::thread::Builder::new()
            .name("contexa-capture".to_string())
            .spawn(move || capture_loop(&running, &exclusion, &tx))
            .map_err(|e| ContexaError::CaptureFailed {
                reason: e.to_string(),
            })?;
        *guard = Some(handle);
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        let handle = {
            let mut guard = self
                .thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.take()
        };
        if let Some(handle) = handle {
            handle.join().map_err(|_| ContexaError::CaptureFailed {
                reason: "capture thread panicked".to_string(),
            })?;
        }
        Ok(())
    }

    fn capture_active_window(&self) -> Result<VisionResult> {
        let (window, frame) = capture_active_frame()?;
        let uia_result = self.extract_uia_text(window.hwnd).ok();
        Ok(build_result(&window, &frame, uia_result))
    }

    fn extract_uia_text(&self, hwnd: isize) -> Result<UiaResult> {
        // Spawns its own short-lived STA thread rather than routing through
        // the persistent capture thread — nothing calls this outside
        // tests/smoke yet, so a request-routing channel would be speculative.
        std::thread::Builder::new()
            .name("contexa-uia-oneshot".to_string())
            .spawn(move || -> Result<UiaResult> {
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                    .ok()
                    .map_err(|e| ContexaError::CaptureFailed {
                        reason: e.to_string(),
                    })?;
                let extractor = UiaExtractor::new()?;
                let result = extractor.extract_text(hwnd);
                unsafe { CoUninitialize() };
                result
            })
            .map_err(|e| ContexaError::CaptureFailed {
                reason: e.to_string(),
            })?
            .join()
            .map_err(|_| ContexaError::CaptureFailed {
                reason: "uia thread panicked".to_string(),
            })?
    }

    fn ocr_region(&self, region: &Region) -> Result<OcrResult> {
        let (_window, frame) = capture_active_frame()?;
        let region = *region;
        // Windows.Media.Ocr is a WinRT/COM API; capture's one-shot thread
        // already uninitialized COM by the time we're back here, so OCR
        // gets its own one-shot COM context (same pattern as `extract_uia_text`).
        std::thread::Builder::new()
            .name("contexa-ocr-oneshot".to_string())
            .spawn(move || -> Result<OcrResult> {
                unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
                    .ok()
                    .map_err(|e| ContexaError::CaptureFailed {
                        reason: e.to_string(),
                    })?;
                let result = OcrEngine::new().ocr_region(&frame, &region);
                unsafe { CoUninitialize() };
                result
            })
            .map_err(|e| ContexaError::CaptureFailed {
                reason: e.to_string(),
            })?
            .join()
            .map_err(|_| ContexaError::CaptureFailed {
                reason: "ocr thread panicked".to_string(),
            })?
    }

    fn get_window_info(&self) -> Result<WindowInfo> {
        WindowMonitor::current_window().ok_or_else(|| ContexaError::CaptureFailed {
            reason: "no foreground window".to_string(),
        })
    }

    fn is_excluded(&self, hwnd: isize) -> bool {
        WindowMonitor::window_info_for(hwnd).is_some_and(|w| self.exclusion.is_excluded(&w))
    }
}

fn build_result(window: &WindowInfo, frame: &Frame, uia_result: Option<UiaResult>) -> VisionResult {
    let frame_hash = ahash(
        &frame.data,
        frame.width as usize,
        frame.height as usize,
        frame.width as usize * 4,
    );
    let capture_method = uia_result.as_ref().map(|_| CaptureMethod::Uia);
    VisionResult {
        hwnd: window.hwnd,
        window_title: window.title.clone(),
        process_name: window.process_name.clone(),
        process_id: window.process_id,
        frame_hash,
        changed_regions: Vec::new(),
        uia_result,
        ocr_result: None,
        capture_method,
        timestamp: frame.timestamp,
    }
}

/// Captures one frame of the current foreground window on a short-lived MTA
/// thread (`CreateFreeThreaded` works on both STA/MTA — ADR-0008/SP-08).
fn capture_active_frame() -> Result<(WindowInfo, Frame)> {
    let window = WindowMonitor::current_window().ok_or_else(|| ContexaError::CaptureFailed {
        reason: "no foreground window".to_string(),
    })?;
    let hwnd = window.hwnd;
    let frame = std::thread::Builder::new()
        .name("contexa-capture-oneshot".to_string())
        .spawn(move || -> Result<Frame> {
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
                .ok()
                .map_err(|e| ContexaError::CaptureFailed {
                    reason: e.to_string(),
                })?;
            let result = FrameCapturer::new().capture_window(hwnd);
            unsafe { CoUninitialize() };
            result
        })
        .map_err(|e| ContexaError::CaptureFailed {
            reason: e.to_string(),
        })?
        .join()
        .map_err(|_| ContexaError::CaptureFailed {
            reason: "capture thread panicked".to_string(),
        })??;
    Ok((window, frame))
}

/// The persistent capture loop (ADR-0008 Pattern A): one STA thread owns the
/// `UIAutomation` instance and the WGC pipeline for the whole loop's lifetime.
fn capture_loop(running: &AtomicBool, exclusion: &ExclusionFilter, tx: &Sender<VisionResult>) {
    if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .is_err()
    {
        tracing::error!("contexa-capture: COM init failed");
        return;
    }

    let uia = UiaExtractor::new().ok();
    let capturer = FrameCapturer::new();
    let monitor = WindowMonitor::new();
    let mut differencer = FrameDifferencer::default();
    let mut scheduler = AdaptiveScheduler::new(Instant::now());

    while running.load(Ordering::SeqCst) {
        let now = Instant::now();
        if let Some(window) = monitor.poll() {
            scheduler.on_focus_change(now);
            if !exclusion.is_excluded(&window) {
                run_capture_tick(
                    &window,
                    &capturer,
                    uia.as_ref(),
                    &mut differencer,
                    &mut scheduler,
                    now,
                    tx,
                );
            }
        }
        let interval = scheduler.tick(now);
        std::thread::sleep(interval);
    }

    unsafe { CoUninitialize() };
}

/// Some target apps (VS Code/Electron) flash their own accessibility focus
/// border on every UIA read of a Document/Edit control. Deferring the walk
/// until typing pauses (state != `Interactive`) turns that into at most one
/// flash per pause instead of one per keystroke — the settled document is
/// what context needs, not every intermediate keystroke.
#[must_use]
fn should_walk_uia(state: CaptureState) -> bool {
    state != CaptureState::Interactive
}

#[allow(clippy::too_many_arguments)]
fn run_capture_tick(
    window: &WindowInfo,
    capturer: &FrameCapturer,
    uia: Option<&UiaExtractor>,
    differencer: &mut FrameDifferencer,
    scheduler: &mut AdaptiveScheduler,
    now: Instant,
    tx: &Sender<VisionResult>,
) {
    let Ok(frame) = capturer.capture_window(window.hwnd) else {
        return;
    };
    let changed = differencer.has_significant_change(&frame);
    scheduler.on_frame_change(now, changed);
    if !changed {
        return;
    }
    // UIA-first, no OCR call from the loop (ADR-0002; OCR is stubbed — see ocr.rs).
    let uia_result = if should_walk_uia(scheduler.state()) {
        uia.and_then(|u| u.extract_text(window.hwnd).ok())
    } else {
        None
    };
    let result = build_result(window, &frame, uia_result);
    send_dropping_oldest(tx, result);
}

/// Bounded channel (docs/05 §9). Docs call for "drop oldest frame" when full,
/// but a `Sender` has no receive capability in crossbeam-channel — reaching
/// in to evict the oldest item would race the real consumer's own `recv()`
/// for the same message. Dropping the newest frame instead is the safe
/// equivalent: the consumer is behind either way, and the frame right after
/// this one will be fresher context than a forcibly-evicted old one would be.
fn send_dropping_oldest(tx: &Sender<VisionResult>, result: VisionResult) {
    match tx.try_send(result) {
        Ok(()) | Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_uia_walk_while_typing_rapidly() {
        assert!(!should_walk_uia(CaptureState::Interactive));
    }

    #[test]
    fn walks_uia_once_settled() {
        assert!(should_walk_uia(CaptureState::Active));
        assert!(should_walk_uia(CaptureState::Idle));
    }
}
