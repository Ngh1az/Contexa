# ADR-0005: Event Bus for Engine Communication

**Status:** Accepted  
**Date:** 2026-07-06  
**Deciders:** Architecture Team

---

## Context

Contexa has multiple independent engines (Vision, Context, Memory, Orchestrator) running on separate threads. They need to communicate without tight coupling or circular dependencies.

Options considered:
- **Direct method calls** — Engines call each other directly
- **Event bus** — Publish/subscribe pattern with typed events
- **Message queue** — External message broker (Redis, etc.)
- **Shared state** — Arc<RwLock> shared between all engines

## Decision

Use an **in-process event bus** with typed events and broadcast channels. The **AI Orchestrator** is the only component that directly coordinates multiple engines for user requests.

## Rationale

| Factor | Event Bus | Direct Calls | Shared State |
|--------|-----------|-------------|--------------|
| Coupling | Loose | Tight | Tight |
| Testability | High (mock events) | Medium | Low |
| Circular deps | Prevented | Risk | N/A |
| Performance | ~μs per event | ~μs | ~μs (lock contention) |
| Complexity | Moderate | Low | Low |
| Scalability | Good (in-process) | Poor | Poor |

The event bus enables the Vision Engine to emit frame results without knowing about the Context Engine. The Context Engine subscribes and processes independently. This matches the pipeline architecture where data flows in one direction.

## Consequences

**Positive:**
- Engines are independently testable with mock event publishers/subscribers
- No circular dependencies between crates
- Easy to add new subscribers (e.g., telemetry) without modifying publishers
- Clear data flow: Vision → Context → Memory

**Negative:**
- Event ordering must be managed (channels are FIFO)
- Debugging event flows is harder than direct calls
- Broadcast channel means all subscribers get all events (filter in subscriber)
- No persistence of events (in-memory only)

## Implementation

```rust
pub enum ContexaEvent {
    VisionFrame(VisionResult),
    ContextUpdate(ContextSnapshot),
    MemoryIndexed { chunk_id: String },
    UserRequest(UserRequest),
    AiResponse(AiResponseChunk),
    ConfigChanged(ConfigDelta),
    Shutdown,
}

pub struct EventBus {
    sender: broadcast::Sender<ContexaEvent>,
}

impl EventBus {
    pub fn publish(&self, event: ContexaEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ContexaEvent> {
        self.sender.subscribe()
    }
}
```

**Exception:** The AI Orchestrator directly calls engine traits for user requests because it needs synchronous coordination and parallel execution via `tokio::join!`.

## References

- [02_System_Architecture.md](../docs/02_System_Architecture.md)
- [19_Coding_Standards.md](../docs/19_Coding_Standards.md)
