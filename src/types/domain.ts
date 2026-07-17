export type ActivePanel = "stats" | "chat" | "feed" | "skin" | "settings" | "memory" | "theme";

export type ChatRole = "user" | "assistant" | "system";

export interface DataPaths {
  root: string;
  database: string;
  settings: string;
  skins: string;
  foodIcons: string;
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

export interface PetStats {
  characterId: string;
  energy: number;
  affection: number;
  todayChatRounds: number;
  todayInteractionCount: number;
  todayHadActivity: boolean;
  reward50Triggered: boolean;
  reward100Triggered: boolean;
  lastChatAt: string | null;
  lastDailySettlementAt: string | null;
  lastWeeklyInventoryClearAt: string | null;
  updatedAt: string;
}

export interface FoodItem {
  id: string;
  characterId: string;
  slotId: number;
  foodLevel: 0 | 1 | 2 | 3;
  displayOrder: number;
  name: string;
  iconPath: string | null;
  energyDelta: number;
  affectionDelta: number;
  category: string;
  rarity: string;
  description: string;
  enabled: boolean;
  count: number;
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

export interface AppSettings {
  apiKey: string;
  provider: string;
  modelName: string;
  autoStart: boolean;
  soundEnabled: boolean;
  ttsEnabled: boolean;
  volume: number;
  notificationPrefs: {
    foodDrops: boolean;
    errors: boolean;
    reminders: boolean;
  };
  privacyLock: boolean;
}

export interface BootstrapPayload {
  dataPaths: DataPaths;
  character: CharacterProfile;
  characters: CharacterProfile[];
  stats: PetStats;
  foods: FoodItem[];
  settings: AppSettings;
  sessions: ChatSession[];
  messages: ChatMessage[];
}

export interface FeedResult {
  food: FoodItem;
  stats: PetStats;
  triggeredStateKey: "feed_neutral" | "feed_blocked";
  message: string;
}

export interface FoodReplaceRequest {
  foodId: string;
  name: string;
  iconDataUrl: string;
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
  enabled?: boolean;
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
}
