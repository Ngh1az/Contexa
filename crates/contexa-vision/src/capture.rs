//! Frame Capturer — `docs/05_Vision_Engine.md` §5.2. Ported from the
//! validated `spikes/SP-02-capture-cpu/src/main.rs` (`Capturer`) and
//! `spikes/SP-08-com-threading/src/main.rs` (`capture_item_for`).
//!
//! `CreateFreeThreaded` works on both STA and MTA (SP-08 note, ADR-0008) —
//! no `DispatcherQueue` needed. Not unit-testable without a live GPU/window
//! (exercised via `examples/vision_smoke.rs`).

use std::ffi::c_void;
use std::time::{Duration, Instant};

use chrono::Utc;
use windows::core::{Interface, Result as WinResult};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use contexa_core::{ContexaError, Result};

use crate::types::Frame;

const FRAME_WAIT_TIMEOUT: Duration = Duration::from_secs(2);

// windows::core::Error is a thin HRESULT wrapper — cheap to take by value,
// and map_err's closure argument arrives owned regardless.
#[allow(clippy::needless_pass_by_value)]
fn win_err(e: windows::core::Error) -> ContexaError {
    ContexaError::CaptureFailed {
        reason: e.to_string(),
    }
}

pub struct FrameCapturer;

impl Default for FrameCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameCapturer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// One-shot capture of `hwnd`. WGC only delivers a frame on content
    /// change (SP-02 note), so this waits up to 2s for one rather than
    /// assuming a frame is immediately ready.
    ///
    /// # Errors
    /// Returns an error if the D3D/WGC pipeline can't be set up, or if no
    /// frame arrives within the timeout.
    pub fn capture_window(&self, hwnd: isize) -> Result<Frame> {
        let hwnd = HWND(hwnd as *mut c_void);
        let mut capturer = Capturer::new(hwnd)?;
        let deadline = Instant::now() + FRAME_WAIT_TIMEOUT;
        loop {
            if let Some(frame) = capturer.grab_frame()? {
                return Ok(frame);
            }
            if Instant::now() > deadline {
                return Err(ContexaError::CaptureFailed {
                    reason: "no frame available within 2s (window idle?)".to_string(),
                });
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}

struct Capturer {
    _device: IDirect3DDevice,
    _session: windows::Graphics::Capture::GraphicsCaptureSession, // must outlive capture
    d3d: ID3D11Device,
    ctx: ID3D11DeviceContext,
    pool: Direct3D11CaptureFramePool,
    staging: Option<ID3D11Texture2D>,
}

impl Capturer {
    fn new(hwnd: HWND) -> Result<Self> {
        let mut device: Option<ID3D11Device> = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        }
        .map_err(win_err)?;
        let d3d = device.ok_or_else(|| ContexaError::CaptureFailed {
            reason: "no d3d11 device".to_string(),
        })?;
        let ctx = unsafe { d3d.GetImmediateContext() }.map_err(win_err)?;
        let dxgi: IDXGIDevice = d3d.cast().map_err(win_err)?;
        let winrt_dev: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
            .map_err(win_err)?
            .cast()
            .map_err(win_err)?;

        let item = capture_item_for(hwnd).map_err(win_err)?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_dev,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            item.Size().map_err(win_err)?,
        )
        .map_err(win_err)?;
        let session = pool.CreateCaptureSession(&item).map_err(win_err)?;
        session.StartCapture().map_err(win_err)?;

        Ok(Self {
            _device: winrt_dev,
            _session: session,
            d3d,
            ctx,
            pool,
            staging: None,
        })
    }

    fn grab_frame(&mut self) -> Result<Option<Frame>> {
        let Ok(frame) = self.pool.TryGetNextFrame() else {
            return Ok(None);
        };
        let surface = frame.Surface().map_err(win_err)?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(win_err)?;
        let tex: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(win_err)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { tex.GetDesc(&mut desc) };

        if self.staging.is_none() {
            let sdesc = D3D11_TEXTURE2D_DESC {
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0.unsigned_abs(),
                MiscFlags: 0,
                ..desc
            };
            let mut st = None;
            unsafe { self.d3d.CreateTexture2D(&sdesc, None, Some(&mut st)) }.map_err(win_err)?;
            self.staging = st;
        }
        let staging = self
            .staging
            .as_ref()
            .ok_or_else(|| ContexaError::CaptureFailed {
                reason: "no staging texture".to_string(),
            })?;
        unsafe { self.ctx.CopyResource(staging, &tex) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.ctx
                .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(win_err)?;
        let pitch = mapped.RowPitch as usize;
        let (width, height) = (desc.Width as usize, desc.Height as usize);
        let row_bytes = width * 4;
        let mut data = vec![0u8; row_bytes * height];
        // SAFETY: `mapped.pData` is valid for `pitch * height` bytes for the
        // lifetime of the Map() call above; we copy out before Unmap().
        unsafe {
            let src = std::slice::from_raw_parts(mapped.pData.cast::<u8>(), pitch * height);
            for y in 0..height {
                data[y * row_bytes..(y + 1) * row_bytes]
                    .copy_from_slice(&src[y * pitch..y * pitch + row_bytes]);
            }
        }
        unsafe { self.ctx.Unmap(staging, 0) };
        frame.Close().ok();

        Ok(Some(Frame {
            data,
            width: desc.Width,
            height: desc.Height,
            timestamp: Utc::now(),
        }))
    }
}

fn capture_item_for(hwnd: HWND) -> WinResult<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForWindow(hwnd) }
}
