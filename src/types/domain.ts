export type ActivePanel = "chat" | "skin" | "settings" | "memory" | "theme" | "help" | "welcome";

export type ChatRole = "user" | "assistant" | "system";

export interface DataPaths {
  root: string;
  database: string;
  settings: string;
  skins: string;
  uploads: string;
  exports: string;
  backups: string;
  logs: string;
}

export interface CharacterProfile {
  id: string;
  name: string;
  skinId: string;
  prompt: string;
  frameAssetsPath: string | null;
  memorySummary: string;
  description: string;
  personalityTags: string[];
  openingLine: string;
  displayScale: number;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChatSession {
  id: string;
  characterId: string;
  title: string;
  summary: string;
  createdAt: string;
  updatedAt: string;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  characterId: string;
  role: ChatRole;
  content: string;
  createdAt: string;
  tokenCount: number;
  metadataJson: string | null;
}

export interface ChatHistoryDay {
  dateKey: string;
  messageCount: number;
  sessionCount: number;
  preview: string;
  lastMessageAt: string;
}

export interface BootstrapPayload {
  dataPaths: DataPaths;
  character: CharacterProfile;
  characters: CharacterProfile[];
  sessions: ChatSession[];
  messages: ChatMessage[];
}

export interface CreateCharacterRequest {
  name: string;
  prompt: string;
  frameSourcePath?: string | null;
}

export interface UpdateCharacterRequest {
  characterId: string;
  name: string;
  prompt: string;
  frameSourcePath?: string | null;
}

export interface SkinValidationIssue {
  severity: "P0" | "P1" | "P2";
  code: string;
  message: string;
  stateKey?: string;
}

export interface SkinValidationReport {
  valid: boolean;
  issues: SkinValidationIssue[];
  frameWidth?: number;
  frameHeight?: number;
}

export interface FrameAssetOption {
  id: string;
  name: string;
  path: string | null;
  builtIn: boolean;
  importedAt: string | null;
  shortActionKeys: string[];
}

export interface ScreenInfo {
  screen_width: number;
  screen_height: number;
  taskbar_height: number;
  scale_factor: number;
  work_area?: { left: number; top: number; right: number; bottom: number };
}

// ==================== AI Provider Types ====================

export interface AiProviderConfig {
  providerId: string;
  providerType: string;
  displayName: string;
  baseUrl: string;
  model: string;
  enabled: boolean;
  stream: boolean;
  temperature: number;
  maxOutputTokens: number;
  timeoutMs: number;
  hasApiKey: boolean;
  apiKeyHint?: string | null;
  isActive: boolean;
  updatedAt: number;
}

export interface SaveAiProviderRequest {
  providerId: string;
  providerType: string;
  displayName: string;
  baseUrl: string;
  model: string;
  apiKey?: string;
  stream?: boolean;
  temperature?: number;
  maxOutputTokens?: number;
  timeoutMs?: number;
}

export interface AiChatEvent {
  event: "start" | "delta" | "done" | "error" | "aborted";
  request_id: string;
  session_id: string;
  content?: string;
  full_response?: string;
  action?: "calm" | "happy" | "sad" | "angry" | "comfort" | "cheer_up" | string;
  message?: string;
  user_message?: ChatMessage;
}
