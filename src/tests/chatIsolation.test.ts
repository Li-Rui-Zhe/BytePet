import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AiChatEvent, BootstrapPayload } from "../types/domain";

const mocks = vi.hoisted(() => ({
  listener: null as null | ((event: { payload: AiChatEvent }) => void),
  listen: vi.fn(), unlisten: vi.fn(), stream: vi.fn(), abort: vi.fn(), switchCharacter: vi.fn(), bootstrap: vi.fn(), scale: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("../storage/database/tauriClient", async (original) => ({
  ...await original<object>(), streamAiMessage: mocks.stream, abortAiRequest: mocks.abort,
  switchCharacter: mocks.switchCharacter, bootstrapApp: mocks.bootstrap, updateCharacterScale: mocks.scale,
}));
import { createMockBootstrap } from "../storage/database/mockData";
import { useAppStore } from "../stores/useAppStore";
import { createRuntimeState } from "../systems/stateMachine/runtime";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
const initial = useAppStore.getState();
function payload(id: string): BootstrapPayload {
  const data = createMockBootstrap();
  return { ...data, character: { ...data.character, id }, characters: [{ ...data.character, id }],
    messages: [], sessions: [{ ...data.sessions[0], characterId: id, id: "session-" + id }] };
}
beforeEach(() => {
  vi.clearAllMocks();
  mocks.listener = null;
  mocks.listen.mockImplementation(async (_name, callback) => { mocks.listener = callback; return mocks.unlisten; });
  mocks.switchCharacter.mockResolvedValue(payload("b"));
  mocks.bootstrap.mockResolvedValue(payload("a"));
  useAppStore.setState({ ...initial, ...payload("a"), onGround: true, currentState: createRuntimeState("calm"),
    movementState: { ...initial.movementState, type: "idle" }, loading: false });
});

describe("chat request isolation", () => {
  it("drains a single queued message after dizziness and never replaces it", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    useAppStore.setState({ currentState: createRuntimeState("dizzy", Date.now() - 10000) });
    await expect(useAppStore.getState().sendAiChat("first")).resolves.toBe("queued");
    await expect(useAppStore.getState().sendAiChat("second")).resolves.toBe("rejected");
    expect(useAppStore.getState().pendingChatMessage).toBe("first");
    useAppStore.getState().tickAnimation();
    useAppStore.getState().tickAnimation();
    await Promise.resolve();
    expect(mocks.stream).toHaveBeenCalledTimes(1);
    expect(mocks.stream.mock.calls[0][3]).toBe("first");
    expect(useAppStore.getState().pendingChatMessage).toBeNull();
    response.resolve(payload("a"));
    await vi.waitFor(() => expect(useAppStore.getState().chatStreaming).toBe(false));
  });

  it("keeps cancelled queued text in a character-specific draft", async () => {
    useAppStore.setState({ currentState: createRuntimeState("dizzy") });
    await useAppStore.getState().sendAiChat("keep this");
    useAppStore.getState().cancelPendingChat();
    expect(useAppStore.getState().chatDrafts.a).toBe("keep this");
    expect(useAppStore.getState().pendingChatMessage).toBeNull();
    await useAppStore.getState().switchCharacter("b");
    expect(useAppStore.getState().chatDrafts.b).toBeUndefined();
    expect(useAppStore.getState().chatDrafts.a).toBe("keep this");
    expect(mocks.stream).not.toHaveBeenCalled();
  });

  it("does not show an unsaved message when preflight fails and keeps the draft", async () => {
    mocks.stream.mockRejectedValue("请填写 DeepSeek API Key。");
    useAppStore.getState().setChatDraft("a", "retry this");
    await expect(useAppStore.getState().sendAiChat("retry this")).resolves.toBe("failed");
    expect(useAppStore.getState().messages).toEqual([]);
    expect(useAppStore.getState().chatDrafts.a).toBe("retry this");
    expect(useAppStore.getState().error).toContain("API Key");
  });

  it("shows a user message only after persistence and restores text after network failure", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    const message = { id: "stored-user", sessionId: "session-a", characterId: "a", role: "user" as const, content: "hello", createdAt: new Date().toISOString(), tokenCount: 5, metadataJson: null };
    useAppStore.getState().setChatDraft("a", "hello");
    const send = useAppStore.getState().sendAiChat("hello");
    await Promise.resolve();
    expect(useAppStore.getState().messages).toEqual([]);
    mocks.listener?.({ payload: { event: "start", request_id: useAppStore.getState().currentRequestId!, session_id: "session-a", user_message: message } });
    expect(useAppStore.getState().messages[0].id).toBe("stored-user");
    expect(useAppStore.getState().chatDrafts.a).toBe("");
    mocks.bootstrap.mockResolvedValue({ ...payload("a"), messages: [message] });
    response.reject("network failed");
    await send;
    expect(useAppStore.getState().messages).toEqual([message]);
    expect(useAppStore.getState().chatDrafts.a).toBe("hello");
  });

  it("does not replace a newly composed draft when the previous request fails", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    const send = useAppStore.getState().sendAiChat("previous message");
    await Promise.resolve();
    useAppStore.getState().setChatDraft("a", "my next message");
    response.reject("network failed");
    await send;
    expect(useAppStore.getState().chatDrafts.a).toBe("my next message");
  });

  it("does not let stale scale responses switch the active character or overwrite a newer preview", async () => {
    const saved = deferred<BootstrapPayload>();
    mocks.scale.mockReturnValue(saved.promise);
    useAppStore.getState().previewCharacterScale("a", 1.5);
    const write = useAppStore.getState().updateCharacterScale("a", 1.5);
    useAppStore.getState().previewCharacterScale("a", 2);
    saved.resolve({ ...payload("a"), characters: [{ ...payload("a").character, displayScale: 1.5 }] });
    await write;
    expect(useAppStore.getState().character?.displayScale).toBe(2);
    expect(useAppStore.getState().body.width).toBe(304);
    const late = deferred<BootstrapPayload>();
    mocks.scale.mockReturnValue(late.promise);
    const oldWrite = useAppStore.getState().updateCharacterScale("a", 2);
    await useAppStore.getState().switchCharacter("b");
    late.resolve({ ...payload("a"), characters: [{ ...payload("a").character, displayScale: 2 }] });
    await oldWrite;
    expect(useAppStore.getState().character?.id).toBe("b");
  });

  it("preserves a scale preview when an older chat snapshot arrives", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    const snapshot = payload("a");
    const send = useAppStore.getState().sendAiChat("hello");
    await Promise.resolve();
    useAppStore.getState().previewCharacterScale("a", 2);
    response.resolve(snapshot);
    await send;
    expect(useAppStore.getState().character?.displayScale).toBe(2);
    expect(useAppStore.getState().characters[0].displayScale).toBe(2);
    expect(useAppStore.getState().body.width).toBe(304);
  });

  it("preserves a saved scale even when an older chat snapshot arrives afterwards", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    const snapshot = payload("a");
    const send = useAppStore.getState().sendAiChat("hello");
    await Promise.resolve();
    useAppStore.getState().previewCharacterScale("a", 1.8);
    const saved = payload("a");
    saved.character.displayScale = 1.8;
    saved.characters[0].displayScale = 1.8;
    mocks.scale.mockResolvedValue(saved);
    await useAppStore.getState().updateCharacterScale("a", 1.8);
    response.resolve(snapshot);
    await send;
    expect(useAppStore.getState().character?.displayScale).toBe(1.8);
    expect(useAppStore.getState().characters[0].displayScale).toBe(1.8);
    expect(useAppStore.getState().body.width).toBe(274);
  });

  it("ignores an older queued save that starts after a newer preview", async () => {
    useAppStore.getState().previewCharacterScale("a", 1.5);
    useAppStore.getState().previewCharacterScale("a", 2);
    const saved = payload("a");
    saved.character.displayScale = 1.5;
    saved.characters[0].displayScale = 1.5;
    mocks.scale.mockResolvedValue(saved);
    await useAppStore.getState().updateCharacterScale("a", 1.5);
    expect(useAppStore.getState().character?.displayScale).toBe(2);
    expect(useAppStore.getState().body.width).toBe(304);
  });

  it("clamps throw targets for the scaled body and treats zero speed as a drop", () => {
    useAppStore.getState().previewCharacterScale("a", 3);
    useAppStore.getState().startDrag(0, 0);
    expect(useAppStore.getState().onGround).toBe(false);
    useAppStore.getState().updateDrag(-10000, -10000);
    useAppStore.getState().endDrag(-1800, -1800);
    const state = useAppStore.getState();
    expect(state.throwTween!.targetX - state.body.width / 2).toBeGreaterThanOrEqual(state.bounds.left);
    expect(state.throwTween!.targetY).toBeGreaterThanOrEqual(state.bounds.top);
    useAppStore.getState().startDrag(0, 0);
    useAppStore.getState().endDrag(0, 0);
    expect(useAppStore.getState().throwTween).toBeNull();
    expect(useAppStore.getState().movementState.type).toBe("fall");
    expect(useAppStore.getState().body.enabled).toBe(true);
  });
  it("registers the listener before sending, binds the character, and rejects duplicate sends", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockImplementation(() => { expect(mocks.listener).not.toBeNull(); return response.promise; });
    const send = useAppStore.getState().sendAiChat("你好");
    await Promise.resolve();
    await useAppStore.getState().sendAiChat("重复");
    expect(mocks.stream).toHaveBeenCalledTimes(1);
    expect(mocks.stream).toHaveBeenCalledWith(expect.any(String), "session-a", "a", "你好");
    response.resolve(payload("a"));
    await send;
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
    expect(useAppStore.getState().chatStreaming).toBe(false);
  });

  it("waits for cancellation before switching, ignoring all stale events and results", async () => {
    const response = deferred<BootstrapPayload>();
    const stop = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    mocks.abort.mockReturnValue(stop.promise);
    const send = useAppStore.getState().sendAiChat("给 A 的消息");
    await Promise.resolve();
    const requestId = useAppStore.getState().currentRequestId!;
    const change = useAppStore.getState().switchCharacter("b");
    expect(mocks.abort).toHaveBeenCalledWith(requestId);
    expect(mocks.switchCharacter).not.toHaveBeenCalled();
    expect(useAppStore.getState().chatStopping).toBe(true);
    mocks.listener?.({ payload: { event: "delta", request_id: requestId, session_id: "session-a", content: "旧消息" } });
    stop.resolve(payload("a"));
    await change;
    mocks.listener?.({ payload: { event: "done", request_id: requestId, session_id: "session-a", full_response: "旧回复", action: "happy" } });
    response.resolve(payload("a"));
    await send;
    expect(useAppStore.getState().character?.id).toBe("b");
    expect(useAppStore.getState().messages).toEqual([]);
    expect(useAppStore.getState().streamingContent).toBe("");
    expect(useAppStore.getState().chatStopping).toBe(false);
  });

  it("does not switch characters if stopping fails", async () => {
    const response = deferred<BootstrapPayload>();
    mocks.stream.mockReturnValue(response.promise);
    mocks.abort.mockRejectedValue("停止失败");
    const send = useAppStore.getState().sendAiChat("你好");
    await Promise.resolve();
    await expect(useAppStore.getState().switchCharacter("b")).rejects.toBe("停止失败");
    expect(mocks.switchCharacter).not.toHaveBeenCalled();
    expect(useAppStore.getState().error).toBe("停止失败");
    response.resolve(payload("a"));
    await send;
  });
});
