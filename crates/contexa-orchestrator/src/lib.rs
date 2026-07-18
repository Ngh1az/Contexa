//! AI Orchestrator — request routing, capability decisions — see `docs/08_AI_Orchestrator.md`

mod decision;
mod engine;
mod pipeline;

pub use decision::DecisionEngine;
pub use engine::{AiOrchestrator, ContexaOrchestrator};
pub use pipeline::{PipelineConfig, PipelineManager};
