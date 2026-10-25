import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import {
  bootstrapApp,
  openDataDir,
  streamAiMessage,
  abortAiRequest as abortAiRequestApi,
  chooseAndImportFrameAsset,
  createCharacter,
  deleteChatHistoryDay,
  deleteCharacter,
  deleteFrameAsset as deleteFrameAssetApi,
  importFrameAssetFromPath,
  listFrameAssets,
  switchCharacter,
  updateCharacter,
  updateCharacterScale as updateCharacterScaleApi,
} from "../storage/database/tauriClient";
import type {
  ActivePanel,
  AiChatEvent,
  BootstrapPayload,
  ChatMessage,
  ChatSession,
  CharacterProfile,
  CreateCharacterRequest,
  DataPaths,
  FrameAssetOption,
  ScreenInfo,
  UpdateCharacterRequest
} from "../types/domain";
import { createRuntimeState, resolveExpiredState } from "../systems/stateMachine/runtime";
import type { RuntimeAnimationState } from "../systems/stateMachine/types";
import {
  createBody,
  createBounds,
  updatePhysics as updatePhysicsEngine,
  type PhysicsBody,
  type WorldBounds,
} from "../systems/physics/engine";
import {
  createInitialMovementState,
  onHitWall,
  tickMovement as tickMovementBehavior,
  getWalkVelocity,
  type MovementState,
} from "../systems/movement/behavior";
import {
  PET_WIDTH,
  PET_HEIGHT,
  THROW_DURATION,
  THROW_PROJECTION_SECONDS,
  FALL_KEY,
  IDLE_KEY,
  YAWN_KEY,
  SIT_KEY,
  SIT_DOWN_KEY,
  SLEEPING_KEY,
  WAKE_UP_KEY,
  MOVEMENT_TICK_MS,
  LIGHT_IDLE_MS,
  SLEEP_IDLE_MS,
  SIT_DOWN_DURATION_MS,
} from "../systems/movement/constants";
import { invoke } from "@tauri-apps/api/core";
import { clampPetPosition, petSize, resizePetBody } from "../systems/physics/petGeometry";

type BehaviorFlow =
  | "idle5_yawn"
  | "idle5_sit"
  | "idle5_sit_down"
  | "sleep_yawn"
  | "post_move_sit"
  | "post_move_sit_down"
  | "short_action"
  | "waking";

interface AppStoreState {
  loading: boolean;
  error: string | null;
  activePanel: ActivePanel | null;
  dataPaths: DataPaths | null;
  character: CharacterProfile | null;
  characters: CharacterProfile[];
  displayScaleOverrides: Record<string, number>;
  frameAssets: FrameAssetOption[];
  sessions: ChatSession[];
  messages: ChatMessage[];
  currentState: RuntimeAnimationState;

  // Physics / movement
  screenInfo: ScreenInfo | null;
  body: PhysicsBody;
  bounds: WorldBounds;
  movementState: MovementState;
  onGround: boolean;
  throwTween: { startX: number; startY: number; targetX: number; targetY: number; startTime: number } | null;

  // Chat / AI state (chatOpen + aiBusy dual lock)
  chatOpen: boolean;
  chatStreaming: boolean;
  chatStopping: boolean;
  streamingContent: string;
  currentRequestId: string | null;
  pendingChatMessage: string | null;
  chatDrafts: Record<string, string>;
  setChatDraft: (characterId: string, text: string) => void;
  cancelPendingChat: () => void;

  // Box button state
  boxPosition: { x: number; y: number } | null;
  boxOpen: boolean;
  toolboxVisible: boolean;

  // Interaction tracking
  lastInteractionAt: number;
  hasTriggered5MinIdle: boolean;
  hasTriggered10MinSleep: boolean;
  behaviorFlow: BehaviorFlow | null;
  sleepIntentQueued: boolean;

  bootstrap: () => Promise<void>;
  openPanel: (panel: ActivePanel) => void;
  closePanel: () => void;
  createCharacter: (request: CreateCharacterRequest) => Promise<void>;
  updateCharacter: (request: UpdateCharacterRequest) => Promise<void>;
  previewCharacterScale: (characterId: string, displayScale: number) => void;
  updateCharacterScale: (characterId: string, displayScale: number) => Promise<void>;
  switchCharacter: (characterId: string) => Promise<void>;
  deleteCharacter: (characterId: string) => Promise<void>;
  deleteChatHistoryDay: (dateKey: string) => Promise<void>;
  loadFrameAssets: () => Promise<void>;
  chooseAndImportFrameAsset: () => Promise<FrameAssetOption | null>;
  importFrameAssetFromPath: (path: string) => Promise<FrameAssetOption>;
  deleteFrameAsset: (assetId: string) => Promise<void>;
  openDataDir: () => Promise<string>;
  tickAnimation: () => void;

  // New physics/movement actions
  initScreenInfo: () => Promise<void>;
  updatePhysics: (dt: number) => void;
  startDrag: (offsetX: number, offsetY: number) => void;
  updateDrag: (x: number, y: number) => void;
  endDrag: (velocityX: number, velocityY: number) => void;
  tickMovement: () => void;

  // Box button actions
  toggleBox: () => void;
  minimizeBox: () => void;
  updateBoxPosition: (x: number, y: number) => void;
  hideToolbox: () => void;
  showToolbox: () => void;

  // AI chat actions (fully implemented in Task #14)
  sendAiChat: (content: string) => Promise<"sent" | "queued" | "rejected" | "failed">;
  abortAiRequest: () => Promise<void>;
  lullToSleep: () => void;
}

let typewriterTimer: ReturnType<typeof setTimeout> | null = null;
let typewriterQueue = "";
let streamDisplayBuffer = "";

const CHAT_ACTIONS = new Set(["calm", "happy", "sad", "angry", "comfort", "cheer_up"]);
const ACTIVE_ACTION_KEYS = new Set(["wake_up", "sleeping", "run_left", "run_right", "fall_down", "fly_up", "sit", "sit_down"]);
const STREAM_ACTION_TAG_BUFFER_CHARS = 128;
const PHYSICAL_STATE_KEYS = new Set([FALL_KEY, "dizzy", "fly_up"]);
const INTERACTION_STATE_KEYS = new Set(["thinking", "happy", "sad", "angry", "comfort", "cheer_up", "error", WAKE_UP_KEY]);
const DEFAULT_SHORT_ACTION_KEYS = ["action1", "action2"];

function canonicalChatAction(action: string | undefined): string | null {
  const normalized = action?.trim().toLowerCase().replace(/[\s-]+/g, "_");
  switch (normalized) {
    case "calm":
    case "clam":
    case "neutral":
    case "idle":
    case "normal":
    case "平静":
    case "普通":
      return "calm";
    case "happy":
    case "joy":
    case "joyful":
    case "开心":
    case "高兴":
    case "快乐":
      return "happy";
    case "sad":
    case "upset":
    case "down":
    case "难过":
    case "伤心":
    case "低落":
      return "sad";
    case "angry":
    case "mad":
    case "annoyed":
    case "生气":
    case "愤怒":
    case "烦躁":
      return "angry";
    case "comfort":
    case "comforting":
    case "support":
    case "supportive":
    case "安慰":
    case "陪伴":
      return "comfort";
    case "cheer_up":
    case "cheer":
    case "encourage":
    case "encouraging":
    case "加油":
    case "鼓励":
    case "打气":
      return "cheer_up";
    default:
      return null;
  }
}

function normalizeChatAction(action: string | undefined): string {
  const canonical = canonicalChatAction(action);
  if (!canonical || !CHAT_ACTIONS.has(canonical)) {
    return "calm";
  }
  return canonical;
}

function canPlayChatAction(state: AppStoreState): boolean {
  if (state.movementState.type === "drag" || state.movementState.type === "thrown" || state.movementState.type === "fall") {
    return false;
  }
  if (state.behaviorFlow === "waking" || state.currentState.definition.key === SLEEPING_KEY || state.currentState.definition.key === WAKE_UP_KEY) {
    return false;
  }
  return !ACTIVE_ACTION_KEYS.has(state.currentState.definition.key);
}

function isPhysicalBusy(state: AppStoreState): boolean {
  return (
    !state.onGround ||
    state.movementState.type === "drag" ||
    state.movementState.type === "thrown" ||
    state.movementState.type === "fall" ||
    Boolean(state.throwTween) ||
    PHYSICAL_STATE_KEYS.has(state.currentState.definition.key)
  );
}

function isInteractionBusy(state: AppStoreState): boolean {
  return state.chatStreaming || INTERACTION_STATE_KEYS.has(state.currentState.definition.key);
}

function canStartPassiveBehavior(state: AppStoreState): boolean {
  return !isPhysicalBusy(state) && !isInteractionBusy(state) && !state.behaviorFlow;
}

function interactionReset(now = Date.now()): Partial<AppStoreState> {
  return {
    lastInteractionAt: now,
    hasTriggered5MinIdle: false,
    hasTriggered10MinSleep: false,
  };
}

function getCurrentFrameAsset(state: AppStoreState): FrameAssetOption | null {
  const currentSkinId = state.character?.skinId ?? "rick_default";
  const currentPath = state.character?.frameAssetsPath ?? null;
  if (currentPath) {
    return state.frameAssets.find((asset) => asset.path === currentPath) ?? null;
  }
  return state.frameAssets.find((asset) => asset.id === currentSkinId) ?? state.frameAssets.find((asset) => asset.id === "rick_default") ?? state.frameAssets[0] ?? null;
}

function getCurrentShortActionKeys(state: AppStoreState): string[] {
  const keys = getCurrentFrameAsset(state)?.shortActionKeys ?? [];
  return keys.length ? keys : DEFAULT_SHORT_ACTION_KEYS;
}

function mergeFrameAsset(current: FrameAssetOption[], asset: FrameAssetOption): FrameAssetOption[] {
  const assetPath = asset.path ? asset.path.toLocaleLowerCase() : null;
  const assetName = asset.name.trim().toLocaleLowerCase();
  const remaining = current.filter((item) => {
    if (item.id === asset.id) return false;
    if (assetPath && item.path?.toLocaleLowerCase() === assetPath) return false;
    if (!asset.builtIn && !item.builtIn && item.name.trim().toLocaleLowerCase() === assetName) return false;
    return true;
  });
  return [asset, ...remaining];
}

function clearTypewriter(): void {
  if (typewriterTimer) {
    clearTimeout(typewriterTimer);
    typewriterTimer = null;
  }
  typewriterQueue = "";
  streamDisplayBuffer = "";
}

function enqueueTypewriter(set: (partial: Partial<AppStoreState> | ((state: AppStoreState) => Partial<AppStoreState>)) => void, content: string): void {
  typewriterQueue += content;
  if (typewriterTimer) return;

  const tick = () => {
    const next = typewriterQueue.slice(0, 2);
    typewriterQueue = typewriterQueue.slice(next.length);
    if (next) {
      set((state) => ({ streamingContent: state.streamingContent + next }));
    }
    if (typewriterQueue) {
      typewriterTimer = setTimeout(tick, 22);
    } else {
      typewriterTimer = null;
    }
  };

  typewriterTimer = setTimeout(tick, 18);
}

function flushVisibleStreamPrefix(
  set: (partial: Partial<AppStoreState> | ((state: AppStoreState) => Partial<AppStoreState>)) => void,
  text: string
): string {
  const visibleLength = Math.max(0, text.length - STREAM_ACTION_TAG_BUFFER_CHARS);
  if (visibleLength <= 0) return text;

  enqueueTypewriter(set, text.slice(0, visibleLength));
  return text.slice(visibleLength);
}

function enqueueVisibleStreamChunk(
  set: (partial: Partial<AppStoreState> | ((state: AppStoreState) => Partial<AppStoreState>)) => void,
  content: string
): void {
  streamDisplayBuffer += content;

  while (true) {
    const lower = streamDisplayBuffer.toLowerCase();
    const toolStart = lower.indexOf("<tool_call");
    if (toolStart < 0) break;

    const toolEnd = lower.indexOf("</tool_call>", toolStart);
    if (toolEnd < 0) {
      const beforeToolCall = streamDisplayBuffer.slice(0, toolStart);
      const pendingToolCall = streamDisplayBuffer.slice(toolStart);
      streamDisplayBuffer = flushVisibleStreamPrefix(set, beforeToolCall) + pendingToolCall;
      return;
    }

    const endIndex = toolEnd + "</tool_call>".length;
    streamDisplayBuffer = streamDisplayBuffer.slice(0, toolStart) + streamDisplayBuffer.slice(endIndex);
  }

  streamDisplayBuffer = flushVisibleStreamPrefix(set, streamDisplayBuffer);
}

function applyBootstrap(set: (partial: Partial<AppStoreState> | ((state: AppStoreState) => Partial<AppStoreState>)) => void, payload: BootstrapPayload): void {
  set((state) => {
    // Local size choices stay authoritative for this session: other IPC calls
    // can return snapshots captured before a preview or even before its save.
    const withLocalScale = (character: CharacterProfile): CharacterProfile => ({
      ...character,
      displayScale: state.displayScaleOverrides[character.id] ?? character.displayScale,
    });
    const character = withLocalScale(payload.character);
    const characters = payload.characters.map(withLocalScale);
    const displayScaleOverrides = Object.fromEntries(
      characters.filter(item => state.displayScaleOverrides[item.id] !== undefined)
        .map(item => [item.id, state.displayScaleOverrides[item.id]])
    );
    return {
      loading: false,
      body: resizePetBody(state.body, state.bounds, character.displayScale),
      dataPaths: payload.dataPaths,
      character,
      characters,
      displayScaleOverrides,
      sessions: payload.sessions,
      messages: payload.messages,
      error: null
    };
  });
}

let chatRevision = 0;
let stoppingPromise: Promise<void> | null = null;
const scaleVersions = new Map<string, number>();

// Default bounds before screen info is loaded
const DEFAULT_BOUNDS: WorldBounds = createBounds(1920, 1080, 60);

export const useAppStore = create<AppStoreState>((set, get) => ({
  loading: true,
  error: null,
  activePanel: null,
  dataPaths: null,
  character: null,
  characters: [],
  displayScaleOverrides: {},
  frameAssets: [{ id: "rick_default", name: "系统默认 Rick", path: null, builtIn: true, importedAt: null, shortActionKeys: DEFAULT_SHORT_ACTION_KEYS }],
  sessions: [],
  messages: [],
  currentState: createRuntimeState("calm"),

  // Physics defaults: pet starts at top-center, falling
  screenInfo: null,
  body: createBody(960, 0, PET_WIDTH, PET_HEIGHT),
  bounds: DEFAULT_BOUNDS,
  movementState: createInitialMovementState(),
  onGround: false,
  throwTween: null,

  // Chat / AI defaults
  chatOpen: false,
  chatStreaming: false,
  chatStopping: false,
  streamingContent: "",
  currentRequestId: null,
  pendingChatMessage: null,
  chatDrafts: {},
  setChatDraft: (id, text) => set((state) => ({ chatDrafts: { ...state.chatDrafts, [id]: text } })),
  cancelPendingChat: () => set({ pendingChatMessage: null }),

  // Box button defaults (null = will be positioned on first render)
  boxPosition: null,
  boxOpen: false,
  toolboxVisible: true,

  // Interaction tracking defaults
  lastInteractionAt: Date.now(),
  hasTriggered5MinIdle: false,
  hasTriggered10MinSleep: false,
  behaviorFlow: null,
  sleepIntentQueued: false,

  bootstrap: async () => {
    try {
      applyBootstrap(set, await bootstrapApp());
      get().loadFrameAssets().catch(() => {});

      // Get screen info and position pet
      await get().initScreenInfo();

      // Read fresh bounds after initScreenInfo
      const freshBounds = get().bounds;
      const size = petSize(get().character?.displayScale ?? 1, freshBounds);
      const startX = freshBounds.left + Math.random() * (freshBounds.right - freshBounds.left - size) + size / 2;
      const startY = freshBounds.top;

      set({
        currentState: createRuntimeState("calm"),
        body: createBody(startX, startY, size, size),
        movementState: createInitialMovementState(),
        onGround: false,
        ...interactionReset(),
        behaviorFlow: null,
        sleepIntentQueued: false,
      });
    } catch (error) {
      set({ loading: false, error: error instanceof Error ? error.message : "启动初始化失败", currentState: createRuntimeState("error") });
    }
  },

  initScreenInfo: async () => {
    try {
      const info = await invoke<ScreenInfo>("get_screen_info");
      const bounds = createBounds(info.screen_width, info.screen_height, info.taskbar_height, info.work_area);
      if (JSON.stringify(info) === JSON.stringify(get().screenInfo)) return;
      set((state) => ({ screenInfo: info, bounds, body: resizePetBody(state.body, bounds, state.character?.displayScale ?? 1) }));
    } catch {
      // Browser preview fallback
      const width = window.screen.width;
      const height = window.screen.height;
      const taskbarHeight = height - window.screen.availHeight;
      const info: ScreenInfo = {
        screen_width: width,
        screen_height: height,
        taskbar_height: taskbarHeight,
        scale_factor: window.devicePixelRatio || 1,
      };
      const bounds = createBounds(width, height, taskbarHeight);
      set((state) => ({ screenInfo: info, bounds, body: resizePetBody(state.body, bounds, state.character?.displayScale ?? 1) }));
    }
  },

  updatePhysics: (dt: number) => {
    const state = get();

    // Handle throw tween
    if (state.throwTween) {
      const tween = state.throwTween;
      const elapsed = Date.now() - tween.startTime;
      const progress = Math.min(elapsed / THROW_DURATION, 1);
      // QuartEaseOut
      const ease = 1 - Math.pow(1 - progress, 4);

      const newX = tween.startX + (tween.targetX - tween.startX) * ease;
      const newY = tween.startY + (tween.targetY - tween.startY) * ease;
      const positioned = clampPetPosition(state.body, state.bounds, newX, newY);

      if (progress >= 1) {
        // Tween complete, enable physics for falling
        set({
          throwTween: null,
          body: {
            ...positioned,
            vx: 0,
            vy: 0,
            enabled: true,
            allowGravity: true,
          },
          movementState: { ...state.movementState, type: "fall", animationKey: FALL_KEY },
          onGround: false,
          currentState: createRuntimeState(FALL_KEY),
        });
        return;
      }

      set({
        body: { ...positioned, enabled: false },
      });
      return;
    }

    // Skip physics during drag
    if (state.movementState.type === "drag" || !state.body.enabled) {
      return;
    }

    // Apply walk velocity
    const body = { ...state.body };
    if (state.movementState.type === "walk_left" || state.movementState.type === "walk_right") {
      body.vx = getWalkVelocity(state.movementState.direction);
      body.allowGravity = false;
      body.vy = 0;
    } else if (state.movementState.type === "fall") {
      body.allowGravity = true;
      body.vx = 0;
    } else {
      // idle / drag / thrown 绛夌姸鎬侊細鍋滄妯Щ
      body.vx = 0;
    }

    // Run physics step
    const collision = updatePhysicsEngine(body, dt, state.bounds);

    const updates: Partial<AppStoreState> = { body };

    // Handle collisions
    if (collision.down && !state.onGround) {
      // Landed!
      body.vx = 0;
      body.vy = 0;
      body.allowGravity = false;

      updates.movementState = {
        ...state.movementState,
        type: "idle",
        animationKey: FALL_KEY,
        idleTimer: state.activePanel ? 999999 : 0,
      };
      updates.onGround = true;
      updates.currentState = createRuntimeState(FALL_KEY);

      // fall_down 4甯14fps 鈮?286ms 鎾畬鍚庡垏 dizzy
      setTimeout(() => {
        const cur = get();
        if (cur.onGround && cur.currentState.definition.key === "fall_down") {
          set({
            currentState: createRuntimeState("dizzy"),
            movementState: {
              ...cur.movementState,
              type: "idle",
              animationKey: "dizzy",
              idleTimer: cur.activePanel ? 999999 : cur.movementState.idleTimer,
            },
          });
          // dizzy durationMs=6000锛岀敱 tickAnimation 鑷姩鍒版湡 鈫?calm
          // dizzy 鍒版湡鍚庯紝tickMovement 浼氳嚜鍔ㄦ仮澶?walk
        }
      }, 400); // fall_down 播放时间
    }

    if (collision.left || collision.right) {
      const side = collision.left ? "left" as const : "right" as const;
      const newMovement = onHitWall(state.movementState, side);
      body.vx = getWalkVelocity(newMovement.direction);
      updates.movementState = newMovement;

      // Only change animation if currently walking
      if (state.movementState.type === "walk_left" || state.movementState.type === "walk_right") {
        updates.currentState = createRuntimeState(newMovement.animationKey);
      }
    }

    if (collision.down && state.onGround && (state.movementState.type === "walk_left" || state.movementState.type === "walk_right")) {
      body.vx = getWalkVelocity(state.movementState.direction);
    }

    set(updates);
  },

  startDrag: (_offsetX: number, _offsetY: number) => {
    set((state) => ({
      ...interactionReset(),
      onGround: false,
      movementState: { ...state.movementState, type: "drag", animationKey: IDLE_KEY },
      throwTween: null,
      body: { ...state.body, enabled: false, vx: 0, vy: 0 },
      currentState: state.currentState.definition.key === SLEEPING_KEY ? createRuntimeState(WAKE_UP_KEY) : createRuntimeState("fly_up"),
      behaviorFlow: null,
      sleepIntentQueued: false,
    }));
  },

  updateDrag: (x: number, y: number) => {
    set((state) => ({
      body: { ...clampPetPosition(state.body, state.bounds, x, y), enabled: false },
    }));
  },

  endDrag: (velocityX: number, velocityY: number) => {
    const state = get();
    const body = clampPetPosition(state.body, state.bounds);
    const vx = Number.isFinite(velocityX) ? velocityX : 0;
    const vy = Number.isFinite(velocityY) ? velocityY : 0;
    const thrown = Math.hypot(vx, vy) >= 80;
    const target = clampPetPosition(body, state.bounds, body.x + vx * THROW_PROJECTION_SECONDS, body.y + vy * THROW_PROJECTION_SECONDS);
    set({
      ...interactionReset(),
      onGround: false,
      body: { ...body, enabled: !thrown, allowGravity: true, vx: 0, vy: 0 },
      throwTween: thrown ? { startX: body.x, startY: body.y, targetX: target.x, targetY: target.y, startTime: Date.now() } : null,
      movementState: { ...state.movementState, type: thrown ? "thrown" : "fall", animationKey: thrown ? "fly_up" : FALL_KEY },
      currentState: createRuntimeState(thrown ? "fly_up" : FALL_KEY),
      behaviorFlow: null,
      sleepIntentQueued: false,
    });
  },

  tickMovement: () => {
    const state = get();
    const now = Date.now();

    // Don't interrupt drag/thrown
    if (state.movementState.type === "drag" || state.movementState.type === "thrown") {
      return;
    }

    // Don't interrupt if not on ground
    if (!state.onGround && state.movementState.type === "fall") {
      return;
    }

    // Don't interrupt dizzy animation (wait for tickAnimation expiry 鈫?calm)
    if (state.currentState.definition.key === "dizzy") {
      return;
    }

    // Don't interrupt fall_down animation
    if (state.currentState.definition.key === "fall_down") {
      return;
    }

    if (state.currentState.definition.key === SLEEPING_KEY || state.currentState.definition.key === WAKE_UP_KEY || state.behaviorFlow) {
      return;
    }

    if (state.sleepIntentQueued && canStartPassiveBehavior(state)) {
      set({
        sleepIntentQueued: false,
        hasTriggered10MinSleep: true,
        behaviorFlow: "sleep_yawn",
        movementState: { ...state.movementState, type: "idle", animationKey: YAWN_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
        currentState: createRuntimeState(YAWN_KEY, now),
      });
      return;
    }

    const idleForMs = now - state.lastInteractionAt;
    if (!state.hasTriggered10MinSleep && idleForMs >= SLEEP_IDLE_MS && canStartPassiveBehavior(state)) {
      set({
        hasTriggered10MinSleep: true,
        behaviorFlow: "sleep_yawn",
        movementState: { ...state.movementState, type: "idle", animationKey: YAWN_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
        currentState: createRuntimeState(YAWN_KEY, now),
      });
      return;
    }

    if (!state.hasTriggered5MinIdle && idleForMs >= LIGHT_IDLE_MS && canStartPassiveBehavior(state)) {
      set({
        hasTriggered5MinIdle: true,
        behaviorFlow: "idle5_yawn",
        movementState: { ...state.movementState, type: "idle", animationKey: YAWN_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
        currentState: createRuntimeState(YAWN_KEY, now),
      });
      return;
    }

    const aiBusy = state.chatStreaming;
    const newMovement = tickMovementBehavior(
      state.movementState,
      MOVEMENT_TICK_MS,
      state.chatOpen,
      aiBusy,
      getCurrentShortActionKeys(state)
    );
    if (newMovement) {
      const body = { ...state.body };
      if (newMovement.type === "walk_left" || newMovement.type === "walk_right") {
        body.vx = getWalkVelocity(newMovement.direction);
        body.allowGravity = false;
      }
      const updates: Record<string, unknown> = { movementState: newMovement, body };
      // 只在动画 key 变化时才重建 RuntimeState，避免 idle→idle 每 111ms 重置帧动画
      if (newMovement.animationKey !== state.currentState.definition.key) {
        updates.currentState = createRuntimeState(newMovement.animationKey);
        if (newMovement.animationKey === SIT_KEY) {
          updates.behaviorFlow = "post_move_sit";
        } else if (/^action\d+$/i.test(newMovement.animationKey)) {
          updates.behaviorFlow = "short_action";
          updates.body = { ...body, vx: 0, vy: 0, allowGravity: false };
        }
      }
      set(updates);
    }
  },

  openPanel: (panel) => {
    set((state) => ({
      ...interactionReset(),
      activePanel: panel,
      boxOpen: false,
      chatOpen: true,
      // 鍋滄绉诲姩锛屾挱鏀?calm
      movementState: { ...state.movementState, type: "idle", animationKey: IDLE_KEY, idleTimer: 999999 },
      body: { ...state.body, vx: 0 },
      currentState:
        state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking"
          ? createRuntimeState(WAKE_UP_KEY)
          : createRuntimeState("calm"),
      behaviorFlow: state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking" ? "waking" : null,
    }));
  },

  closePanel: () => set((state) => ({
    activePanel: null,
    boxOpen: false,
    chatOpen: false,
    // 閲嶇疆 idle 璁?tickMovement 鎭㈠鑷富绉诲姩
    movementState: state.movementState.type === "idle"
      ? { ...state.movementState, idleTimer: 0 }
      : state.movementState,
  })),

  createCharacter: async (request) => {
    await get().abortAiRequest();
    ++chatRevision;
    set({ pendingChatMessage: null });
    applyBootstrap(set, await createCharacter(request));
    set({ currentState: createRuntimeState("calm") });
  },

  updateCharacter: async (request) => {
    applyBootstrap(set, await updateCharacter(request));
    set({ currentState: createRuntimeState("calm") });
  },

  previewCharacterScale: (characterId, displayScale) => {
    scaleVersions.set(characterId, (scaleVersions.get(characterId) ?? 0) + 1);
    const scale = Math.min(3, Math.max(0.75, Number.isFinite(displayScale) ? displayScale : 1));
    set((state) => ({
      displayScaleOverrides: { ...state.displayScaleOverrides, [characterId]: scale },
      body: state.character?.id === characterId ? resizePetBody(state.body, state.bounds, scale) : state.body,
      character:
        state.character?.id === characterId
          ? { ...state.character, displayScale: scale }
          : state.character,
      characters: state.characters.map((item) =>
        item.id === characterId ? { ...item, displayScale: scale } : item
      ),
    }));
  },

  updateCharacterScale: async (characterId, displayScale) => {
    const scale = Math.min(3, Math.max(0.75, Number.isFinite(displayScale) ? displayScale : 1));
    const version = scaleVersions.get(characterId) ?? 0;
    try {
      const payload = await updateCharacterScaleApi(characterId, scale);
      if ((scaleVersions.get(characterId) ?? 0) !== version) return;
      const latestScale = get().displayScaleOverrides[characterId];
      // A queued older save may only start after a newer preview already exists.
      if (latestScale !== undefined && latestScale !== scale) return;
      const saved = payload.characters.find(item => item.id === characterId);
      if (!saved) return;
      set((state) => ({
        displayScaleOverrides: { ...state.displayScaleOverrides, [characterId]: saved.displayScale },
        characters: state.characters.map(item => item.id === characterId ? { ...item, displayScale: saved.displayScale } : item),
        character: state.character?.id === characterId ? { ...state.character, displayScale: saved.displayScale } : state.character,
        body: state.character?.id === characterId ? resizePetBody(state.body, state.bounds, saved.displayScale) : state.body,
      }));
    } catch (error) {
      set({ error: typeof error === "string" ? error : error instanceof Error ? error.message : "角色大小保存失败，请重试。" });
      throw error;
    }
  },

  switchCharacter: async (characterId) => {
    await get().abortAiRequest();
    ++chatRevision;
    set({ pendingChatMessage: null });
    applyBootstrap(set, await switchCharacter(characterId));
    set({ currentState: createRuntimeState("calm") });
  },

  deleteCharacter: async (characterId) => {
    await get().abortAiRequest();
    ++chatRevision;
    set({ pendingChatMessage: null });
    applyBootstrap(set, await deleteCharacter(characterId));
    set({ currentState: createRuntimeState("calm") });
  },

  deleteChatHistoryDay: async (dateKey) => {
    await get().abortAiRequest();
    ++chatRevision;
    applyBootstrap(set, await deleteChatHistoryDay(dateKey));
  },

  loadFrameAssets: async () => {
    set({ frameAssets: await listFrameAssets() });
  },

  chooseAndImportFrameAsset: async () => {
    const asset = await chooseAndImportFrameAsset();
    if (asset) {
      const frameAssets = await listFrameAssets();
      set({
        frameAssets: mergeFrameAsset(frameAssets, asset),
      });
    }
    return asset;
  },

  importFrameAssetFromPath: async (path) => {
    const asset = await importFrameAssetFromPath(path);
    const frameAssets = await listFrameAssets();
    set({
      frameAssets: mergeFrameAsset(frameAssets, asset),
    });
    return asset;
  },

  deleteFrameAsset: async (assetId) => {
    const frameAssets = await deleteFrameAssetApi(assetId);
    set({ frameAssets });
  },

  openDataDir: () => openDataDir(),

  tickAnimation: () => {
    const state = get();
    if (state.pendingChatMessage && !isPhysicalBusy(state) && !state.chatStreaming && !state.chatStopping) {
      const text = state.pendingChatMessage;
      set({ pendingChatMessage: null });
      void get().sendAiChat(text);
      return;
    }
    const now = Date.now();
    const key = state.currentState.definition.key;
    const expired = state.currentState.expiresAt !== null && now >= state.currentState.expiresAt;

    if (state.sleepIntentQueued && canStartPassiveBehavior(state)) {
      set({
        sleepIntentQueued: false,
        hasTriggered10MinSleep: true,
        behaviorFlow: "sleep_yawn",
        movementState: { ...state.movementState, type: "idle", animationKey: YAWN_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
        currentState: createRuntimeState(YAWN_KEY, now),
      });
      return;
    }

    if (!expired) {
      if (state.behaviorFlow === "waking" && key !== WAKE_UP_KEY) {
        set({ behaviorFlow: null });
      }
      return;
    }

    if (state.behaviorFlow === "waking" && key === WAKE_UP_KEY) {
      set({
        behaviorFlow: null,
        currentState: createRuntimeState(IDLE_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: IDLE_KEY, idleTimer: 0 },
      });
      return;
    }

    if (state.behaviorFlow === "waking") {
      set({ behaviorFlow: null });
      return;
    }

    if (state.behaviorFlow === "sleep_yawn" && key === YAWN_KEY) {
      set({
        behaviorFlow: null,
        currentState: createRuntimeState(SLEEPING_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: SLEEPING_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
      });
      return;
    }

    if (state.behaviorFlow === "idle5_yawn" && key === YAWN_KEY) {
      set({
        behaviorFlow: "idle5_sit",
        currentState: createRuntimeState(SIT_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: SIT_KEY, idleTimer: SIT_DOWN_DURATION_MS },
      });
      return;
    }

    if ((state.behaviorFlow === "idle5_sit" || state.behaviorFlow === "post_move_sit") && key === SIT_KEY) {
      set({
        behaviorFlow: state.behaviorFlow === "idle5_sit" ? "idle5_sit_down" : "post_move_sit_down",
        currentState: createRuntimeState(SIT_DOWN_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: SIT_DOWN_KEY, idleTimer: SIT_DOWN_DURATION_MS },
      });
      return;
    }

    if ((state.behaviorFlow === "idle5_sit_down" || state.behaviorFlow === "post_move_sit_down") && key === SIT_DOWN_KEY) {
      set({
        behaviorFlow: null,
        currentState: createRuntimeState(IDLE_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: IDLE_KEY, idleTimer: 0 },
      });
      return;
    }

    if (state.behaviorFlow === "short_action") {
      set({
        behaviorFlow: null,
        currentState: createRuntimeState(IDLE_KEY, now),
        movementState: { ...state.movementState, type: "idle", animationKey: IDLE_KEY, idleTimer: 0 },
      });
      return;
    }

    set({ currentState: resolveExpiredState(state.currentState, "calm", now) });
  },

  // ==================== Box Button Actions ====================

  toggleBox: () => {
    set((state) => ({
      ...interactionReset(),
      activePanel: state.activePanel ? null : state.activePanel,
      boxOpen: state.activePanel ? false : !state.boxOpen,
      chatOpen: state.activePanel ? false : state.chatOpen,
      currentState:
        state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking"
          ? createRuntimeState(WAKE_UP_KEY)
          : state.currentState,
      behaviorFlow: state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking" ? "waking" : state.behaviorFlow,
      movementState:
        state.activePanel && state.movementState.type === "idle"
          ? { ...state.movementState, idleTimer: 0 }
          : state.movementState,
    }));
  },

  minimizeBox: () => {
    set({
      ...interactionReset(),
      boxOpen: false,
    });
  },

  updateBoxPosition: (x: number, y: number) => {
    set({ boxPosition: { x, y } });
  },

  hideToolbox: () => {
    set((state) => ({
      ...interactionReset(),
      toolboxVisible: false,
      boxOpen: false,
      activePanel: state.activePanel ? null : state.activePanel,
      chatOpen: state.activePanel ? false : state.chatOpen,
    }));
  },

  showToolbox: () => {
    set({ toolboxVisible: true });
  },

  // ==================== AI Chat Actions ====================

  sendAiChat: async (content: string) => {
    const text = content.trim();
    const state = get();
    if (!text || !state.character || state.chatStreaming || state.chatStopping) return "rejected";
    if (state.pendingChatMessage) { set({ error: "已有一条消息等待发送，请先取消等待再编辑。" }); return "rejected"; }
    if ([...text].length > 16000) { set({ error: "消息过长，请缩短到 16000 字以内。" }); return "rejected"; }
    if (isPhysicalBusy(state)) {
      set((s) => ({ pendingChatMessage: text, error: null, chatDrafts: { ...s.chatDrafts, [state.character!.id]: text } }));
      return "queued";
    }
    const requestId = crypto.randomUUID();
    const characterId = state.character.id;
    const sessionId = state.sessions[0]?.id ?? crypto.randomUUID();
    const revision = ++chatRevision;
    const isCurrent = () => chatRevision === revision && get().character?.id === characterId;
    const restoreDraft = () => set((s) => ({ chatDrafts: { ...s.chatDrafts, [characterId]: s.chatDrafts[characterId] || text } }));
    const waking = state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking";
    clearTypewriter();
    set({ ...interactionReset(), chatStreaming: true, error: null, streamingContent: "", currentRequestId: requestId,
      currentState: createRuntimeState(waking ? WAKE_UP_KEY : "thinking"),
      behaviorFlow: waking ? "waking" : state.behaviorFlow,
      movementState: waking ? { ...state.movementState, type: "idle", animationKey: WAKE_UP_KEY, idleTimer: 999999 } : state.movementState,
    });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<AiChatEvent>("ai-chat-event", ({ payload: evt }) => {
        if (evt.request_id !== requestId || !isCurrent() || get().currentRequestId !== requestId) return;
        if (evt.event === "delta") { enqueueVisibleStreamChunk(set, evt.content ?? ""); return; }
        if (evt.event === "start") {
          if (evt.user_message) {
            const message = evt.user_message;
            set((s) => ({
              messages: s.messages.some(item => item.id === message.id) ? s.messages : [...s.messages, message],
              chatDrafts: { ...s.chatDrafts, [characterId]: s.chatDrafts[characterId]?.trim() === text ? "" : s.chatDrafts[characterId] ?? "" },
            }));
          }
          return;
        }
        clearTypewriter();
        set({ chatStreaming: false, streamingContent: "", currentRequestId: null });
        if (evt.event === "done") {
          const action = normalizeChatAction(evt.action);
          set((s) => ({ messages: [...s.messages, { id: crypto.randomUUID(), sessionId, characterId, role: "assistant",
            content: evt.full_response ?? "", createdAt: new Date().toISOString(), tokenCount: 0,
            metadataJson: JSON.stringify({ action, rawAction: evt.action ?? null }) }],
            currentState: canPlayChatAction(s) ? createRuntimeState(action) : s.currentState }));
        } else if (evt.event === "error") {
          restoreDraft();
          set({ error: evt.message ?? "AI 请求失败，请检查聊天连接设置。", currentState: createRuntimeState("error") });
        } else { set({ currentState: createRuntimeState("calm") }); }
      });
      if (!isCurrent() || get().currentRequestId !== requestId) return "rejected";
      const payload = await streamAiMessage(requestId, sessionId, characterId, text);
      if (isCurrent()) {
        applyBootstrap(set, payload);
        set((s) => ({ chatStreaming: false, streamingContent: "", currentRequestId: null, chatDrafts: { ...s.chatDrafts, [characterId]: s.chatDrafts[characterId]?.trim() === text ? "" : s.chatDrafts[characterId] ?? "" } }));
      }
      return "sent";
    } catch (error) {
      if (isCurrent()) {
        clearTypewriter();
        restoreDraft();
        // Reconcile persisted history; a preflight failure never creates a fake sent message.
        try {
          const payload = await bootstrapApp();
          if (isCurrent()) applyBootstrap(set, payload);
        } catch { /* Keep the last known history if refreshing also fails. */ }
        if (!isCurrent()) return "failed";
        set({ error: error instanceof Error ? error.message : typeof error === "string" ? error : "AI 请求失败，请检查聊天连接设置。",
          chatStreaming: false, chatStopping: false, streamingContent: "", currentRequestId: null, currentState: createRuntimeState("error") });
      }
      return "failed";
    } finally { unlisten?.(); }
  },

  abortAiRequest: async () => {
    if (stoppingPromise) return stoppingPromise;
    const { currentRequestId: requestId, character } = get();
    set({ pendingChatMessage: null });
    if (!requestId) return;
    const revision = ++chatRevision;
    clearTypewriter();
    set({ chatStreaming: true, chatStopping: true, currentRequestId: null, streamingContent: "" });
    stoppingPromise = (async () => {
      let stopped = false;
      try {
        const payload = await abortAiRequestApi(requestId);
        stopped = true;
        if (chatRevision === revision && get().character?.id === character?.id) applyBootstrap(set, payload);
      } catch (error) {
        set({ error: typeof error === "string" ? error : error instanceof Error ? error.message : "停止请求失败，请重试。" });
        throw error;
      } finally {
        if (chatRevision === revision) set({ chatStreaming: !stopped, chatStopping: false, currentRequestId: stopped ? null : requestId, currentState: createRuntimeState(stopped ? "calm" : "error") });
        stoppingPromise = null;
      }
    })();
    return stoppingPromise;
  },

  lullToSleep: () => {
    const state = get();
    const now = Date.now();

    if (state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "sleep_yawn") {
      set({ boxOpen: false });
      return;
    }

    if (isPhysicalBusy(state) || isInteractionBusy(state)) {
      set({
        ...interactionReset(now),
        boxOpen: false,
        sleepIntentQueued: true,
      });
      return;
    }

    set({
      ...interactionReset(now),
      boxOpen: false,
      activePanel: null,
      chatOpen: false,
      hasTriggered10MinSleep: true,
      behaviorFlow: "sleep_yawn",
      sleepIntentQueued: false,
      movementState: { ...state.movementState, type: "idle", animationKey: YAWN_KEY, idleTimer: 999999 },
      body: { ...state.body, vx: 0, vy: 0, allowGravity: false },
      currentState: createRuntimeState(YAWN_KEY, now),
    });
  },
}));
