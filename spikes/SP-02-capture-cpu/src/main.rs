// SP-02: Graphics Capture + frame diff CPU (docs/22 §4).
// Capture loop (WGC free-threaded pool) at 1/5/10 fps; per frame: copy to staging,
// downsample to 16x16 grid at 1/4 resolution, 256-bit average hash, hamming diff vs
// previous frame (>5% = "changed"). CPU via GetProcessTimes sampled every 5 s.
// Targets: idle 1fps <1%, active 5fps <3%, interactive 10fps <5% (task-manager style,
// i.e. divided by logical core count); memory < 100 MB.
// Usage: sp02-capture-cpu [seconds_per_state]  (default 60; spec full run = 1800)

use anyhow::{bail, Context, Result};
use std::time::{Duration, Instant};
use windows::core::Interface;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{FILETIME, HWND};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

const GRID: usize = 16;

fn ft_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

fn process_cpu_100ns() -> u64 {
    let (mut c, mut e, mut k, mut u) = Default::default();
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) }.ok();
    ft_to_u64(k) + ft_to_u64(u)
}

/// 256-bit average hash from BGRA rows, sampling at 1/4 resolution.
fn ahash(data: &[u8], width: usize, height: usize, pitch: usize) -> [u64; 4] {
    let mut cells = [0u32; GRID * GRID];
    let mut counts = [0u32; GRID * GRID];
    let (sw, sh) = (width / 4, height / 4); // 1/4 resolution sampling
    for sy in 0..sh {
        let y = sy * 4;
        let row = &data[y * pitch..];
        let cy = sy * GRID / sh.max(1);
        for sx in 0..sw {
            let x = sx * 4;
            let px = &row[x * 4..x * 4 + 3];
            let lum = (px[0] as u32 + px[1] as u32 * 2 + px[2] as u32) / 4; // cheap luma
            let cx = sx * GRID / sw.max(1);
            let idx = (cy.min(GRID - 1)) * GRID + cx.min(GRID - 1);
            cells[idx] += lum;
            counts[idx] += 1;
        }
    }
    let mut avg_all = 0u64;
    let mut vals = [0u32; GRID * GRID];
    for i in 0..GRID * GRID {
        vals[i] = cells[i] / counts[i].max(1);
        avg_all += vals[i] as u64;
    }
    let avg = (avg_all / (GRID * GRID) as u64) as u32;
    let mut hash = [0u64; 4];
    for i in 0..GRID * GRID {
        if vals[i] > avg {
            hash[i / 64] |= 1 << (i % 64);
        }
    }
    hash
}

fn hamming(a: &[u64; 4], b: &[u64; 4]) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
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
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )?
        };
        let d3d = device.context("no d3d device")?;
        let ctx = unsafe { d3d.GetImmediateContext() }?;
        let dxgi: IDXGIDevice = d3d.cast()?;
        let winrt_dev: IDirect3DDevice =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }?.cast()?;

        let interop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd) }?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_dev,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            item.Size()?,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;
        Ok(Self { _device: winrt_dev, _session: session, d3d, ctx, pool, staging: None })
    }

    /// Grab latest frame if available, return its hash.
    fn grab_hash(&mut self) -> Result<Option<[u64; 4]>> {
        let frame = match self.pool.TryGetNextFrame() {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };
        let surface = frame.Surface()?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
        let tex: ID3D11Texture2D = unsafe { access.GetInterface() }?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { tex.GetDesc(&mut desc) };

        if self.staging.is_none() {
            let sdesc = D3D11_TEXTURE2D_DESC {
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
                ..desc
            };
            let mut st = None;
            unsafe { self.d3d.CreateTexture2D(&sdesc, None, Some(&mut st)) }?;
            self.staging = st;
        }
        let staging = self.staging.as_ref().unwrap();
        unsafe { self.ctx.CopyResource(staging, &tex) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { self.ctx.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }?;
        let pitch = mapped.RowPitch as usize;
        let data = unsafe {
            std::slice::from_raw_parts(mapped.pData as *const u8, pitch * desc.Height as usize)
        };
        let hash = ahash(data, desc.Width as usize, desc.Height as usize, pitch);
        unsafe { self.ctx.Unmap(staging, 0) };
        frame.Close().ok();
        Ok(Some(hash))
    }
}

fn run_state(cap: &mut Capturer, name: &str, fps: u64, secs: u64, cores: u64) -> Result<(f64, f64)> {
    let interval = Duration::from_micros(1_000_000 / fps);
    let start = Instant::now();
    let cpu_start = process_cpu_100ns();
    let mut prev: Option<[u64; 4]> = None;
    let (mut frames, mut skipped, mut changed) = (0u64, 0u64, 0u64);
    let mut cpu_samples: Vec<f64> = Vec::new();
    let mut last_sample = (Instant::now(), process_cpu_100ns());

    while start.elapsed() < Duration::from_secs(secs) {
        let tick = Instant::now();
        if let Some(h) = cap.grab_hash()? {
            frames += 1;
            if let Some(p) = &prev {
                let diff_pct = hamming(p, &h) as f64 / 256.0 * 100.0;
                if diff_pct > 5.0 {
                    changed += 1;
                } else {
                    skipped += 1; // production would skip UIA/OCR here
                }
            }
            prev = Some(h);
        }
        // 5-second CPU sampling
        if last_sample.0.elapsed() >= Duration::from_secs(5) {
            let now_cpu = process_cpu_100ns();
            let wall = last_sample.0.elapsed().as_secs_f64();
            let cpu_pct = (now_cpu - last_sample.1) as f64 / 10_000_000.0 / wall * 100.0;
            cpu_samples.push(cpu_pct / cores as f64);
            last_sample = (Instant::now(), now_cpu);
        }
        if let Some(rem) = interval.checked_sub(tick.elapsed()) {
            std::thread::sleep(rem);
        }
    }
    let total_cpu =
        (process_cpu_100ns() - cpu_start) as f64 / 10_000_000.0 / start.elapsed().as_secs_f64() * 100.0;
    let avg_task_mgr = total_cpu / cores as f64;
    let mem = memory_stats::memory_stats().map(|m| m.physical_mem as f64 / 1_048_576.0).unwrap_or(0.0);
    println!(
        "{name}: {fps} fps × {secs}s → frames={frames} changed={changed} skipped={skipped} | CPU {avg_task_mgr:.2}% of machine ({total_cpu:.2}% single-core) | mem {mem:.0} MB",
    );
    Ok((avg_task_mgr, mem))
}

fn main() -> Result<()> {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(60);
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        bail!("no foreground window");
    }
    let cores = std::thread::available_parallelism()?.get() as u64;
    println!("hwnd {:?}, {} logical cores, {}s per state\n", hwnd, cores, secs);

    let mut cap = Capturer::new(hwnd)?;
    let (idle, _) = run_state(&mut cap, "idle       ", 1, secs, cores)?;
    let (active, _) = run_state(&mut cap, "active     ", 5, secs, cores)?;
    let (inter, mem) = run_state(&mut cap, "interactive", 10, secs, cores)?;

    println!("\n=== SP-02 gate (targets: idle<1%, active<3%, interactive<5%, mem<100MB) ===");
    assert!(idle < 1.0, "GATE FAIL: idle CPU {idle:.2}% >= 1%");
    assert!(active < 3.0, "GATE FAIL: active CPU {active:.2}% >= 3%");
    assert!(inter < 5.0, "GATE FAIL: interactive CPU {inter:.2}% >= 5%");
    assert!(mem < 100.0, "GATE FAIL: memory {mem:.0} MB >= 100 MB");
    println!("GATE: PASS");
    Ok(())
}
