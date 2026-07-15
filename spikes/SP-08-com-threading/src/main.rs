// SP-08: COM threading model validation (docs/22 §10, ADR-0008).
// Patterns:
//   A: single spawned thread, CoInitializeEx(COINIT_APARTMENTTHREADED) — UIA + capture together
//   B: capture thread (MTA) + UIA thread (STA), running in parallel
//   C: all operations on main thread (STA)
// Each pattern runs N cycles of { UIA ElementFromHandle+CurrentName } and N captured frames.
// Pass: zero COM errors, no deadlocks (watchdog), throughput documented.

use anyhow::{bail, Context, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::Interface;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED,
};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

const CYCLES: u64 = 1000;

struct Stats {
    uia_ok: u64,
    uia_err: u64,
    frames: u64,
    frame_err: u64,
    uia_ms: u128,
    cap_ms: u128,
}

fn uia_cycles(hwnd: HWND, n: u64) -> Result<(u64, u64, u128)> {
    let auto: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
            .context("CoCreateInstance(CUIAutomation)")?;
    let (mut ok, mut err) = (0u64, 0u64);
    let t = Instant::now();
    for _ in 0..n {
        let r = (|| -> Result<()> {
            let el = unsafe { auto.ElementFromHandle(hwnd) }?;
            let _name = unsafe { el.CurrentName() }?;
            Ok(())
        })();
        match r {
            Ok(()) => ok += 1,
            Err(_) => err += 1,
        }
    }
    Ok((ok, err, t.elapsed().as_millis()))
}

fn make_d3d() -> Result<IDirect3DDevice> {
    let mut device: Option<ID3D11Device> = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            Default::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )?
    };
    let device = device.context("no d3d11 device")?;
    let dxgi: IDXGIDevice = device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }?;
    Ok(inspectable.cast()?)
}

fn capture_item_for(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    let interop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    Ok(unsafe { interop.CreateForWindow(hwnd) }?)
}

/// Capture n frames using a free-threaded frame pool (works on both STA and MTA).
fn capture_cycles(hwnd: HWND, n: u64) -> Result<(u64, u64, u128)> {
    let device = make_d3d()?;
    let item = capture_item_for(hwnd)?;
    let size = item.Size()?;
    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        size,
    )?;
    let session = pool.CreateCaptureSession(&item)?;
    // ponytail: yellow-border/cursor settings left default — not the spike's question
    session.StartCapture()?;

    let (mut frames, mut errs) = (0u64, 0u64);
    let t = Instant::now();
    let deadline = Instant::now() + Duration::from_secs(120);
    while frames < n {
        match pool.TryGetNextFrame() {
            Ok(frame) => {
                let _ = frame.SystemRelativeTime(); // touch the frame
                frame.Close().ok();
                frames += 1;
            }
            Err(_) => {
                errs += 1;
            }
        }
        if Instant::now() > deadline {
            bail!("capture deadline exceeded (possible deadlock): {frames} frames");
        }
        // WGC delivers at display refresh; don't spin at 100% between frames
        std::thread::sleep(Duration::from_micros(500));
    }
    let ms = t.elapsed().as_millis();
    session.Close().ok();
    pool.Close().ok();
    Ok((frames, errs, ms))
}

fn run_pattern_a(hwnd: isize) -> Result<Stats> {
    // Single spawned STA thread doing UIA then capture
    std::thread::spawn(move || -> Result<Stats> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        let hwnd = HWND(hwnd as *mut _);
        let (uia_ok, uia_err, uia_ms) = uia_cycles(hwnd, CYCLES)?;
        let (frames, frame_err, cap_ms) = capture_cycles(hwnd, CYCLES)?;
        unsafe { CoUninitialize() };
        Ok(Stats { uia_ok, uia_err, frames, frame_err, uia_ms, cap_ms })
    })
    .join()
    .map_err(|_| anyhow::anyhow!("pattern A thread panicked"))?
}

fn run_pattern_b(hwnd: isize) -> Result<Stats> {
    // Capture thread (MTA) + UIA thread (STA) in parallel
    let progress = Arc::new(AtomicU64::new(0));
    let p2 = progress.clone();

    let cap = std::thread::spawn(move || -> Result<(u64, u64, u128)> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        let r = capture_cycles(HWND(hwnd as *mut _), CYCLES);
        unsafe { CoUninitialize() };
        p2.fetch_add(1, Ordering::SeqCst);
        r
    });
    let uia = std::thread::spawn(move || -> Result<(u64, u64, u128)> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        let r = uia_cycles(HWND(hwnd as *mut _), CYCLES);
        unsafe { CoUninitialize() };
        r
    });

    let (frames, frame_err, cap_ms) =
        cap.join().map_err(|_| anyhow::anyhow!("capture thread panicked"))??;
    let (uia_ok, uia_err, uia_ms) =
        uia.join().map_err(|_| anyhow::anyhow!("uia thread panicked"))??;
    Ok(Stats { uia_ok, uia_err, frames, frame_err, uia_ms, cap_ms })
}

fn run_pattern_c(hwnd: isize) -> Result<Stats> {
    // Everything on the main thread, STA
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let hwnd = HWND(hwnd as *mut _);
    let (uia_ok, uia_err, uia_ms) = uia_cycles(hwnd, CYCLES)?;
    let (frames, frame_err, cap_ms) = capture_cycles(hwnd, CYCLES)?;
    unsafe { CoUninitialize() };
    Ok(Stats { uia_ok, uia_err, frames, frame_err, uia_ms, cap_ms })
}

fn report(name: &str, s: &Stats) {
    let uia_tput = s.uia_ok as f64 / (s.uia_ms.max(1) as f64 / 1000.0);
    let cap_fps = s.frames as f64 / (s.cap_ms.max(1) as f64 / 1000.0);
    println!(
        "pattern {name}: UIA {}/{} ok ({} errs) in {} ms ({:.0} ops/s) | capture {} frames ({} empty polls) in {} ms ({:.0} fps)",
        s.uia_ok, CYCLES, s.uia_err, s.uia_ms, uia_tput, s.frames, s.frame_err, s.cap_ms, cap_fps
    );
}

fn main() -> Result<()> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        bail!("no foreground window");
    }
    let hwnd_val = hwnd.0 as isize;
    println!("target hwnd: {:?}, cycles per pattern: {}", hwnd, CYCLES);

    let a = run_pattern_a(hwnd_val)?;
    report("A (single spawned STA)", &a);
    let b = run_pattern_b(hwnd_val)?;
    report("B (capture MTA + UIA STA)", &b);
    let c = run_pattern_c(hwnd_val)?;
    report("C (main thread STA)", &c);

    // Gate: zero COM errors on UIA path, capture produced all frames, no deadlock (we got here)
    for (name, s) in [("A", &a), ("B", &b), ("C", &c)] {
        assert_eq!(s.uia_err, 0, "GATE FAIL: pattern {name} had UIA COM errors");
        assert_eq!(s.frames, CYCLES, "GATE FAIL: pattern {name} incomplete capture");
    }
    println!("\nGATE: PASS — no COM errors, no deadlocks across A/B/C");
    Ok(())
}
