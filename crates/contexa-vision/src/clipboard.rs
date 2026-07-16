//! Clipboard access — `docs/06_Context_Engine.md` §5.5 Selection Tracker
//! fallback ("clipboard monitoring, if user copies"). Plain Win32, no COM/UIA
//! needed. Not unit-testable (real OS clipboard); exercised via
//! `examples/vision_smoke.rs`.

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Changes every time clipboard content changes — cheap to poll, so callers
/// can skip re-reading the clipboard when this hasn't moved.
#[must_use]
pub fn sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

/// Reads the clipboard as Unicode text. `None` if the clipboard is empty,
/// holds non-text content, or can't be opened (e.g. another app has it locked).
#[must_use]
pub fn read_text() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let text = read_unicode_text();
        let _ = CloseClipboard();
        text
    }
}

unsafe fn read_unicode_text() -> Option<String> {
    let handle: HANDLE = GetClipboardData(CF_UNICODETEXT.0.into()).ok()?;
    let ptr = GlobalLock(windows::Win32::Foundation::HGLOBAL(handle.0));
    if ptr.is_null() {
        return None;
    }
    let wide_ptr = ptr.cast::<u16>();
    let len = wide_strlen(wide_ptr);
    let text = String::from_utf16_lossy(std::slice::from_raw_parts(wide_ptr, len));
    let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(handle.0));
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// `CF_UNICODETEXT` is a null-terminated wide string, not length-prefixed.
unsafe fn wide_strlen(ptr: *const u16) -> usize {
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    len
}
