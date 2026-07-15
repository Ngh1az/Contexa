//! Window Monitor — `docs/05_Vision_Engine.md` §5.1. Polls the foreground
//! window; real Win32 calls, not unit-testable (exercised via
//! `examples/vision_smoke.rs`).

use std::ffi::c_void;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;

use chrono::Utc;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

use crate::types::WindowInfo;

pub struct WindowMonitor {
    last_hwnd: AtomicIsize,
    pub poll_interval: Duration,
}

impl Default for WindowMonitor {
    fn default() -> Self {
        Self {
            last_hwnd: AtomicIsize::new(0),
            poll_interval: Duration::from_millis(100),
        }
    }
}

impl WindowMonitor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the current foreground window, or `None` if there isn't one.
    #[must_use]
    pub fn current_window() -> Option<WindowInfo> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return None;
        }
        Self::window_info_for(hwnd.0 as isize)
    }

    /// Reads title/process info for an arbitrary `hwnd` (not necessarily the
    /// foreground window) — used by `is_excluded(hwnd)`.
    #[must_use]
    pub fn window_info_for(hwnd: isize) -> Option<WindowInfo> {
        let handle = HWND(hwnd as *mut c_void);
        let title = window_title(handle);
        let mut process_id = 0u32;
        unsafe { GetWindowThreadProcessId(handle, Some(&mut process_id)) };
        let process_name = process_name(process_id).unwrap_or_default();
        Some(WindowInfo {
            hwnd,
            title,
            process_id,
            process_name,
            timestamp: Utc::now(),
        })
    }

    /// Returns `Some(window)` if the foreground HWND changed since the last poll.
    pub fn poll(&self) -> Option<WindowInfo> {
        let window = Self::current_window()?;
        let prev = self.last_hwnd.swap(window.hwnd, Ordering::SeqCst);
        if prev == window.hwnd {
            return None;
        }
        Some(window)
    }
}

fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..usize::try_from(len).unwrap_or(0)])
}

fn process_name(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 512];
    let mut len: u32 = 512;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(handle);
    }
    result.ok()?;
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}
