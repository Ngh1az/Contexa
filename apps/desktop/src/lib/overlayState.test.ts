import { describe, expect, it } from "vitest";
import { initialOverlayState, overlayReducer } from "./overlayState";

describe("overlayReducer", () => {
  it("submit appends a user message and an empty assistant placeholder, moves to processing", () => {
    const next = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hello" });
    expect(next.phase).toBe("processing");
    expect(next.requestId).toBe("r1");
    expect(next.error).toBeNull();
    expect(next.messages).toEqual([
      { id: "r1-user", role: "user", content: "hello" },
      { id: "r1", role: "assistant", content: "" },
    ]);
  });

  it("a second submit appends onto existing messages (multi-turn)", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "first" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    state = overlayReducer(state, { type: "submit", requestId: "r2", query: "second" });
    expect(state.messages).toHaveLength(4);
    expect(state.messages[2]).toEqual({ id: "r2-user", role: "user", content: "second" });
  });

  it("first chunk moves processing to streaming and appends content to the assistant message", () => {
    const processing = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    const next = overlayReducer(processing, { type: "chunk", requestId: "r1", content: "Hel" });
    expect(next.phase).toBe("streaming");
    expect(next.messages[1].content).toBe("Hel");
  });

  it("chunks accumulate in order", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "Hel" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "lo" });
    expect(state.messages[1].content).toBe("Hello");
  });

  it("chunk from a stale request id is ignored", () => {
    const state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    const next = overlayReducer(state, { type: "chunk", requestId: "stale", content: "x" });
    expect(next).toBe(state);
  });

  it("complete returns to idle while keeping the accumulated message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    expect(state.phase).toBe("idle");
    expect(state.messages[1].content).toBe("answer");
  });

  it("error from the in-flight request surfaces as a banner and returns to idle, keeping partial message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "partial" });
    const next = overlayReducer(state, { type: "error", requestId: "r1", message: "provider down" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("provider down");
    expect(next.messages[1].content).toBe("partial");
  });

  it("rejected (synchronous handle_request failure) surfaces a banner without touching existing messages", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "first" });
    state = overlayReducer(state, { type: "complete", requestId: "r1" });
    const messagesBefore = state.messages;
    const next = overlayReducer(state, { type: "rejected", reason: "unknown action" });
    expect(next.phase).toBe("idle");
    expect(next.error).toBe("unknown action");
    expect(next.messages).toBe(messagesBefore);
  });

  it("cancel returns to idle and clears the request id but keeps partial message content", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "partial" });
    state = overlayReducer(state, { type: "cancel" });
    expect(state.phase).toBe("idle");
    expect(state.requestId).toBeNull();
    expect(state.messages[1].content).toBe("partial");
  });

  it("reset returns to the initial state (overlay reopened)", () => {
    let state = overlayReducer(initialOverlayState, { type: "submit", requestId: "r1", query: "hi" });
    state = overlayReducer(state, { type: "chunk", requestId: "r1", content: "answer" });
    expect(overlayReducer(state, { type: "reset" })).toEqual(initialOverlayState);
  });
});
