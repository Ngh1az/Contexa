//! Built-in `ContextEnricher` implementations — `docs/18_Plugin_System.md` §6.

mod chromium;
mod vscode;

pub use chromium::ChromiumEnricher;
pub use vscode::VsCodeEnricher;
