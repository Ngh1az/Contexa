import { describe, expect, it } from "vitest";
import { initialOverlayState, overlayReducer } from "./overlayState";

describe("overlayReducer", () => {
  it("submit moves to processing and clears prior response", () => {
    const prior = { ...initialOverlayState, response: "old answer" };
    const next = overlayReducer(prior, { type: "submit", requestId: "r1" });
    expect(next).toEqual({ phase: "processing", requestId: "r1", response: "", error: null });
  });

  it("first chunk moves processing to streaming and appends content", () => {
    const processing = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    const next = overlayReducer(processing, { type: "chunk", requestId: "r1", content: "Hel" });
    expect(next.phase).toBe("streaming");
    expect(next.response).toBe("Hel");
  });

  it("chunks accumulate in order", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "Hel" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "lo" });
    expect(state.response).toBe("Hello");
  });

  it("chunk from a stale request id is ignored", () => {
    const state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    const next = overlayReducer(state, { type: "chunk", requestId: "stale", content: "x" });
    expect(next).toBe(state);
  });

  it("complete returns to idle while keeping the accumulated response", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    expect(state.phase).toBe("idle");
    expect(state.response).toBe("answer");
  });

  it("error from the in-flight request surfaces the message and returns to idle", () => {
    const state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    const next = overlayReducer(state, { type: "error", requestId: "r1", message: "provider down" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("provider down");
  });

  it("rejected (synchronous handle_request failure) surfaces error without touching requestId", () => {
    const next = overlayReducer(initialOverlayState, { type: "rejected", reason: "unknown action" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("unknown action");
  });

  it("cancel returns to idle and clears the request id but keeps partial response", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "partial" });
    state = overlayReducer(state, { type: "cancel" });
    expect(state).toEqual({ phase: "idle", requestId: null, response: "partial", error: null });
  });

  it("reset returns to the initial state (overlay reopened)", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    expect(overlayReducer(state, { type: "reset" })).toEqual(initialOverlayState);
  });
});
