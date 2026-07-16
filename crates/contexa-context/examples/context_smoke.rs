//! Manual smoke test — NOT part of `cargo test`. Run interactively against
//! whatever window currently has focus:
//!
//! ```powershell
//! cargo run -p contexa-context --example context_smoke
//! ```
//!
//! Mirrors `crates/contexa-vision/examples/vision_smoke.rs`: needs a live
//! foreground window and display session that an automated test run can't
//! guarantee. Captures one `VisionResult` from whatever window has focus,
//! runs it through the full Context Engine pipeline (assembler -> built-in
//! enrichers -> selection tracker -> language detector -> change detector ->
//! cache), and prints the resulting `ContextSnapshot`. Focus Chrome/Edge to
//! see `url` populated, or VS Code on a git repo to see `document_path` +
//! workspace metadata (see `enrichers::vscode` doc comment for why there's
//! no git branch yet). Select some text (or copy something) before running
//! to see `selected_text` populated.

use contexa_context::{ContexaContextEngine, ContextEngine};
use contexa_core::ContextSnapshot;
use contexa_vision::{ContexaVisionEngine, VisionEngine};

fn main() {
    let (vision, _rx) = ContexaVisionEngine::new(Vec::new());
    let context = ContexaContextEngine::with_builtin_enrichers();
    context.enable_selection_tracking();

    let result = match vision.capture_active_window() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("capture_active_window failed: {e}");
            return;
        }
    };
    println!(
        "captured: process={} title={:?}",
        result.process_name, result.window_title
    );

    match context.process_vision_result(result) {
        Ok(Some(snapshot)) => print_snapshot(&snapshot),
        Ok(None) => println!("no meaningful change detected (unexpected on first call)"),
        Err(e) => eprintln!("process_vision_result failed: {e}"),
    }
}

fn print_snapshot(snapshot: &ContextSnapshot) {
    println!("--- ContextSnapshot ---");
    println!("id: {}", snapshot.id);
    println!("window_title: {:?}", snapshot.window_title);
    println!("process_name: {}", snapshot.process_name);
    println!("url: {:?}", snapshot.url);
    println!("document_path: {:?}", snapshot.document_path);
    println!("selected_text: {:?}", snapshot.selected_text);
    println!("language: {:?}", snapshot.language);
    println!("capture_method: {:?}", snapshot.capture_method);
    println!("metadata: {:?}", snapshot.metadata);
    let text_len = snapshot.visible_text.as_ref().map_or(0, String::len);
    println!("visible_text: {text_len} chars");
}
