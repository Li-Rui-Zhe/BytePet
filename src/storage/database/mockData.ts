import type { BootstrapPayload, CharacterProfile, ChatMessage, ChatSession } from "../../types/domain";

const now = new Date().toISOString();

export const mockCharacter: CharacterProfile = {
  id: "rick-default-character",
  name: "Rick",
  skinId: "rick_default",
  prompt: "你是 Rick，一个安静、可靠、带一点好奇心的桌宠。你会用简短、温和的方式陪用户聊天。",
  frameAssetsPath: null,
  memorySummary: "",
  description: "你是 Rick，一个安静、可靠、带一点好奇心的桌宠。你会用简短、温和的方式陪用户聊天。",
  personalityTags: ["温和", "好奇", "可靠"],
  openingLine: "嗨，我是 Rick。今天也一起慢慢来。",
  displayScale: 1,
  isActive: true,
  createdAt: now,
  updatedAt: now,
};

export const mockSession: ChatSession = {
  id: "mock-session",
  characterId: mockCharacter.id,
  title: "新聊天",
  summary: "",
  createdAt: now,
  updatedAt: now,
};

export const mockMessages: ChatMessage[] = [
  {
    id: "mock-opening",
    sessionId: mockSession.id,
    characterId: mockCharacter.id,
    role: "assistant",
    content: mockCharacter.openingLine,
    createdAt: now,
    tokenCount: 0,
    metadataJson: null,
  },
];

export function createMockBootstrap(): BootstrapPayload {
  return {
    dataPaths: {
      root: "Tauri appDataDir/bytepet-data",
      database: "Tauri appDataDir/bytepet-data/bytepet.db",
      settings: "settings",
      skins: "skins",
      uploads: "uploads",
      exports: "exports",
      backups: "backups",
      logs: "logs",
    },
    character: mockCharacter,
    characters: [mockCharacter],
    sessions: [mockSession],
    messages: mockMessages,
  };
}
