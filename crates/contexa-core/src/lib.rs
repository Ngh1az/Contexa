//! Shared types, event bus, and config — see `docs/02_System_Architecture.md`

mod error;
mod types;

pub use error::{ContexaError, Result};
pub use types::{
    CaptureMethod, ContextSnapshot, ExecutionPlan, RequestAction, RequestHandle,
    RequestPreferences, RequestStatus, UserRequest,
};
