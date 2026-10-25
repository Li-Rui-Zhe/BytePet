import { invoke } from "@tauri-apps/api/core";
import type {
  AiProviderConfig,
  BootstrapPayload,
  ChatHistoryDay,
  ChatMessage,
  CreateCharacterRequest,
  FrameAssetOption,
  SaveAiProviderRequest,
  UpdateCharacterRequest
} from "../../types/domain";
import { createMockBootstrap } from "./mockData";

function hasTauriRuntime(): boolean {
  return Boolean(window.__TAURI_INTERNALS__);
}

async function invokeOrMock<T>(command: string, args: Record<string, unknown>, fallback: () => T | Promise<T>): Promise<T> {
  if (!hasTauriRuntime()) {
    return fallback();
  }

  return invoke<T>(command, args);
}

let mockState = createMockBootstrap();

export async function bootstrapApp(): Promise<BootstrapPayload> {
  return invokeOrMock("bootstrap_app", {}, () => {
    mockState = createMockBootstrap();
    return mockState;
  });
}

export async function listFrameAssets(): Promise<FrameAssetOption[]> {
  return invokeOrMock("list_frame_assets", {}, () => [
    { id: "rick_default", name: "系统默认 Rick", path: null, builtIn: true, importedAt: null, shortActionKeys: ["action1", "action2"] },
  ]);
}

export async function chooseAndImportFrameAsset(): Promise<FrameAssetOption | null> {
  return invokeOrMock("choose_and_import_frame_asset", {}, () => null);
}

export async function importFrameAssetFromPath(path: string): Promise<FrameAssetOption> {
  const id = path.split(/[\\/]/).filter(Boolean).at(-1) ?? crypto.randomUUID();
  return invokeOrMock("import_frame_asset_from_path", { path }, () => ({
    id,
    name: id,
    path: null,
    builtIn: false,
    importedAt: new Date().toISOString(),
    shortActionKeys: ["action1"],
  }));
}

export async function deleteFrameAsset(assetId: string): Promise<FrameAssetOption[]> {
  return invokeOrMock("delete_frame_asset", { assetId }, () => {
    if (assetId === "rick_default") throw new Error("系统默认 Rick 素材包不能删除。");
    return [{ id: "rick_default", name: "系统默认 Rick", path: null, builtIn: true, importedAt: null, shortActionKeys: ["action1", "action2"] }];
  });
}

export async function createCharacter(request: CreateCharacterRequest): Promise<BootstrapPayload> {
  return invokeOrMock("create_character", { request }, () => {
    const now = new Date().toISOString();
    const character = {
      id: crypto.randomUUID(),
      name: request.name.trim(),
      skinId: request.frameSourcePath?.trim() || "rick_default",
      prompt: request.prompt.trim(),
      frameAssetsPath: null,
      memorySummary: "",
      description: request.prompt.trim(),
      personalityTags: [],
      openingLine: `你好，我是${request.name.trim()}。`,
      displayScale: 1,
      isActive: true,
      createdAt: now,
      updatedAt: now,
    };
    const session = {
      id: crypto.randomUUID(),
      characterId: character.id,
      title: "新聊天",
      summary: "",
      createdAt: now,
      updatedAt: now,
    };
    const message: ChatMessage = {
      id: crypto.randomUUID(),
      sessionId: session.id,
      characterId: character.id,
      role: "assistant",
      content: character.openingLine,
      createdAt: now,
      tokenCount: 0,
      metadataJson: null,
    };
    mockState = {
      ...mockState,
      character,
      characters: [
        character,
        ...mockState.characters.map((item) => ({ ...item, isActive: false })),
      ],
      sessions: [session],
      messages: [message],
    };
    return mockState;
  });
}

export async function updateCharacter(request: UpdateCharacterRequest): Promise<BootstrapPayload> {
  return invokeOrMock("update_character", { request }, () => {
    const now = new Date().toISOString();
    mockState = {
      ...mockState,
      characters: mockState.characters.map((item) =>
        item.id === request.characterId
          ? {
              ...item,
              name: request.name.trim(),
              skinId: request.frameSourcePath?.trim() || item.skinId,
              prompt: request.prompt.trim(),
              description: request.prompt.trim(),
              frameAssetsPath: null,
              updatedAt: now,
            }
          : item
      ),
    };
    mockState = {
      ...mockState,
      character: mockState.characters.find((item) => item.id === request.characterId && item.isActive) ?? mockState.character,
    };
    return mockState;
  });
}

export async function updateCharacterScale(characterId: string, displayScale: number): Promise<BootstrapPayload> {
  return invokeOrMock("update_character_scale", { characterId, displayScale }, () => {
    const scale = Math.min(3, Math.max(0.75, displayScale));
    const now = new Date().toISOString();
    mockState = {
      ...mockState,
      characters: mockState.characters.map((item) =>
        item.id === characterId ? { ...item, displayScale: scale, updatedAt: now } : item
      ),
      character:
        mockState.character?.id === characterId
          ? { ...mockState.character, displayScale: scale, updatedAt: now }
          : mockState.character,
    };
    return mockState;
  });
}

export async function switchCharacter(characterId: string): Promise<BootstrapPayload> {
  return invokeOrMock("switch_character", { characterId }, () => {
    const character = mockState.characters.find((item) => item.id === characterId);
    if (!character) throw new Error("人物不存在");
    mockState = {
      ...mockState,
      character: { ...character, isActive: true },
      characters: mockState.characters.map((item) => ({ ...item, isActive: item.id === characterId })),
      sessions: mockState.sessions.filter((session) => session.characterId === characterId),
      messages: mockState.messages.filter((message) => message.characterId === characterId),
    };
    return mockState;
  });
}

export async function deleteCharacter(characterId: string): Promise<BootstrapPayload> {
  return invokeOrMock("delete_character", { characterId }, () => {
    if (mockState.characters.length <= 1) throw new Error("至少保留一个人物");
    const remaining = mockState.characters.filter((item) => item.id !== characterId);
    const next = remaining[0];
    mockState = {
      ...mockState,
      character: { ...next, isActive: true },
      characters: remaining.map((item, index) => ({ ...item, isActive: index === 0 })),
      sessions: mockState.sessions.filter((session) => session.characterId === next.id),
      messages: mockState.messages.filter((message) => message.characterId === next.id),
    };
    return mockState;
  });
}

function mockHistoryDays(): ChatHistoryDay[] {
  const groups = new Map<string, ChatMessage[]>();
  for (const message of mockState.messages.filter((item) => item.characterId === mockState.character.id)) {
    const key = message.createdAt.slice(0, 10);
    groups.set(key, [...(groups.get(key) ?? []), message]);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => b.localeCompare(a))
    .map(([dateKey, dayMessages]) => {
      const sorted = [...dayMessages].sort((a, b) => a.createdAt.localeCompare(b.createdAt));
      const last = sorted[sorted.length - 1];
      return {
        dateKey,
        messageCount: sorted.length,
        sessionCount: new Set(sorted.map((message) => message.sessionId)).size,
        preview: last?.content ?? "",
        lastMessageAt: last?.createdAt ?? `${dateKey}T00:00:00`,
      };
    });
}

export async function chatHistoryDays(): Promise<ChatHistoryDay[]> {
  return invokeOrMock("chat_history_days", {}, () => mockHistoryDays());
}

export async function chatHistoryMessagesForDay(dateKey: string): Promise<ChatMessage[]> {
  return invokeOrMock("chat_history_messages_for_day", { dateKey }, () =>
    mockState.messages
      .filter((message) => message.characterId === mockState.character.id && message.createdAt.startsWith(dateKey))
      .sort((a, b) => a.createdAt.localeCompare(b.createdAt))
  );
}

export async function deleteChatHistoryDay(dateKey: string): Promise<BootstrapPayload> {
  return invokeOrMock("delete_chat_history_day", { dateKey }, () => {
    const deletedSessionIds = new Set(
      mockState.messages
        .filter((message) => message.characterId === mockState.character.id && message.createdAt.startsWith(dateKey))
        .map((message) => message.sessionId)
    );
    mockState = {
      ...mockState,
      messages: mockState.messages.filter(
        (message) => !(message.characterId === mockState.character.id && message.createdAt.startsWith(dateKey))
      ),
    };
    const remainingSessionIds = new Set(mockState.messages.map((message) => message.sessionId));
    mockState = {
      ...mockState,
      sessions: mockState.sessions.filter((session) => !deletedSessionIds.has(session.id) || remainingSessionIds.has(session.id)),
    };
    return mockState;
  });
}

export async function openDataDir(): Promise<string> {
  return invokeOrMock("open_data_dir", {}, () => mockState.dataPaths.root);
}

// ==================== AI Provider Functions ====================

export async function getAiConfigPublic(): Promise<AiProviderConfig[]> {
  return invokeOrMock("get_ai_config_public", {}, () => []);
}

export async function saveAiProviderConfig(config: SaveAiProviderRequest): Promise<AiProviderConfig> {
  return invoke("save_ai_provider_config", { config });
}

export async function deleteAiConnection(providerId: string): Promise<void> {
  return invoke("delete_ai_connection", { providerId });
}

export async function setActiveAiProvider(providerId: string): Promise<void> {
  return invoke("set_active_ai_provider", { providerId });
}

export async function testAiConnection(config: SaveAiProviderRequest): Promise<string> {
  return invoke("test_ai_connection", { config });
}

export async function streamAiMessage(requestId: string, sessionId: string, characterId: string, content: string): Promise<BootstrapPayload> {
  return invoke("stream_ai_message", { requestId, sessionId, characterId, content });
}

export async function abortAiRequest(requestId: string): Promise<BootstrapPayload> {
  return invoke("abort_ai_request", { requestId });
}
