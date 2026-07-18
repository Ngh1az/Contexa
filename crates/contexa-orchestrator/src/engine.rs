//! `ContexaOrchestrator` — `docs/08_AI_Orchestrator.md` §8, §10.
//!
//! `AiOrchestrator::handle_request` returns only a `RequestHandle` (no
//! stream) per the spec — the actual token stream has to reach the caller
//! some other way. `take_stream` (not part of the trait; a pragmatic
//! addition) hands back the `ResponseStream` once the pipeline has produced
//! one, via a oneshot channel the caller awaits — the same "hand back a raw
//! channel for the composition root to drive" pattern already used by
//! `contexa-vision`'s capture thread and `contexa-context`'s `subscribe()`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::{oneshot, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;

use contexa_context::ContextEngine;
use contexa_core::{ContexaError, RequestHandle, RequestStatus, Result, UserRequest};
use contexa_llm::ResponseStream;

use crate::decision::DecisionEngine;
use crate::pipeline::PipelineManager;

// docs/08 §10: "Maximum 3 concurrent AI requests to prevent resource exhaustion."
const DEFAULT_CONCURRENCY_LIMIT: usize = 3;

#[async_trait]
pub trait AiOrchestrator: Send + Sync {
    /// # Errors
    /// Returns an error if no context is available to plan against.
    async fn handle_request(&self, request: UserRequest) -> Result<RequestHandle>;
    /// # Errors
    /// Returns an error if `request_id` is unknown.
    async fn cancel_request(&self, request_id: &str) -> Result<()>;
    fn get_active_requests(&self) -> Vec<RequestHandle>;
}

struct RequestState {
    handle: RequestHandle,
    join: Option<JoinHandle<()>>,
    stream_rx: Option<oneshot::Receiver<ResponseStream>>,
}

pub struct ContexaOrchestrator {
    context: Arc<dyn ContextEngine>,
    decision: DecisionEngine,
    pipeline: Arc<PipelineManager>,
    semaphore: Arc<Semaphore>,
    requests: Arc<Mutex<HashMap<String, RequestState>>>,
}

impl ContexaOrchestrator {
    #[must_use]
    pub fn new(context: Arc<dyn ContextEngine>, pipeline: Arc<PipelineManager>) -> Self {
        Self {
            context,
            decision: DecisionEngine,
            pipeline,
            semaphore: Arc::new(Semaphore::new(DEFAULT_CONCURRENCY_LIMIT)),
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Awaits the `ResponseStream` for a request already accepted by
    /// `handle_request`. Resolves to `None` if the request was cancelled,
    /// failed before producing a stream, or `request_id` is unknown.
    pub async fn take_stream(&self, request_id: &str) -> Option<ResponseStream> {
        let rx = {
            let mut guard = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
            guard.get_mut(request_id)?.stream_rx.take()
        }?;
        rx.await.ok()
    }
}

#[async_trait]
impl AiOrchestrator for ContexaOrchestrator {
    async fn handle_request(&self, request: UserRequest) -> Result<RequestHandle> {
        let context = request
            .context_override
            .clone()
            .or_else(|| self.context.get_current())
            .ok_or(ContexaError::ContextUnavailable)?;
        let plan = self.decision.decide(&request, &context);

        let id = Uuid::new_v4().to_string();
        let handle = RequestHandle {
            id: id.clone(),
            status: RequestStatus::Planning,
            started_at: Utc::now(),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
            guard.insert(
                id.clone(),
                RequestState {
                    handle: handle.clone(),
                    join: None,
                    stream_rx: Some(rx),
                },
            );
        }

        let semaphore = Arc::clone(&self.semaphore);
        let pipeline = Arc::clone(&self.pipeline);
        let requests = Arc::clone(&self.requests);
        let task_id = id.clone();

        let join = tokio::spawn(async move {
            let Ok(_permit) = semaphore.acquire_owned().await else {
                return;
            };
            set_status_on(&requests, &task_id, RequestStatus::Gathering);
            match pipeline.execute(plan, request).await {
                Ok(stream) => {
                    set_status_on(&requests, &task_id, RequestStatus::Generating);
                    let _ = tx.send(stream); // receiver dropped if the request was cancelled
                }
                Err(e) => {
                    set_status_on(&requests, &task_id, RequestStatus::Failed(e.to_string()));
                }
            }
        });

        if let Some(state) = self
            .requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(&id)
        {
            state.join = Some(join);
        }

        Ok(handle)
    }

    async fn cancel_request(&self, request_id: &str) -> Result<()> {
        let mut guard = self.requests.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(state) = guard.get_mut(request_id) else {
            return Err(ContexaError::Conversion(format!(
                "unknown request id: {request_id}"
            )));
        };
        if let Some(join) = state.join.take() {
            join.abort();
        }
        state.handle.status = RequestStatus::Cancelled;
        state.stream_rx = None;
        Ok(())
    }

    fn get_active_requests(&self) -> Vec<RequestHandle> {
        self.requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|s| s.handle.clone())
            .collect()
    }
}

// Free function (not `&self.set_status`) because it's called from inside the
// spawned task, which only holds `Arc` clones, not `&ContexaOrchestrator`.
fn set_status_on(requests: &Mutex<HashMap<String, RequestState>>, id: &str, status: RequestStatus) {
    let mut guard = requests.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(state) = guard.get_mut(id) {
        state.handle.status = status;
    }
}
