//! Manual smoke test — NOT part of `cargo test`. Run interactively against
//! whatever window currently has focus:
//!
//! ```powershell
//! cargo run -p contexa-vision --example vision_smoke
//! ```
//!
//! Mirrors how `spikes/SP-01`/`SP-02`/`SP-08` were verified: this needs a
//! live foreground window and display session that an automated test run
//! can't guarantee, so it's a manual example rather than a `#[test]`.

use contexa_vision::{ContexaVisionEngine, VisionEngine};

fn main() {
    let (engine, _rx) = ContexaVisionEngine::new(Vec::new());

    match engine.get_window_info() {
        Ok(info) => println!(
            "foreground window: hwnd={} title={:?} process={} (pid {})",
            info.hwnd, info.title, info.process_name, info.process_id
        ),
        Err(e) => {
            eprintln!("get_window_info failed: {e}");
            return;
        }
    }

    match engine.capture_active_window() {
        Ok(result) => {
            println!(
                "capture_active_window: frame_hash={:?} capture_method={:?}",
                result.frame_hash, result.capture_method
            );
            match &result.uia_result {
                Some(uia) => {
                    println!(
                        "  UIA: {} chars, {} elements, depth {}, confidence {:.2}, {} ms",
                        uia.text.len(),
                        uia.element_count,
                        uia.tree_depth,
                        uia.confidence,
                        uia.duration_ms
                    );
                    let sample: String = uia.text.chars().take(200).collect();
                    println!("  sample: {}", sample.replace('\n', " | "));
                }
                None => println!("  UIA: no result (extraction failed)"),
            }
        }
        Err(e) => eprintln!("capture_active_window failed: {e}"),
    }

    match engine.ocr_region(&contexa_vision::Region {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
    }) {
        Ok(_) => {
            println!(
                "ocr_region: unexpectedly succeeded (SP-03 was supposed to be a prerequisite)"
            );
        }
        Err(e) => println!("ocr_region: {e} (expected — see docs/22)"),
    }
}
