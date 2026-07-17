import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import {
  bootstrapApp,
  feedFood,
  openDataDir,
  recordPetInteraction,
  reorderFoods,
  replaceFood,
  resetStats,
  saveSettings,
  saveInterruptedAiMessage,
  sendChatMessage,
  validateSkinPath,
  streamAiMessage,
  abortAiRequest as abortAiRequestApi,
  chooseAndImportFrameAsset,
  createCharacter,
  deleteChatSession,
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
  AppSettings,
  BootstrapPayload,
  ChatMessage,
  ChatSession,
  CharacterProfile,
  CreateCharacterRequest,
  DataPaths,
  FeedResult,
  FoodReplaceRequest,
  FoodItem,
  FrameAssetOption,
  PetStats,
  ScreenInfo,
  SkinValidationReport,
  UpdateCharacterRequest
} from "../types/domain";
import { createRuntimeState, resolveExpiredState, transitionTo } from "../systems/stateMachine/runtime";
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
  TWEEN_ACCELERATION,
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
import { partitionStreamBuffer } from "../systems/chat/streamFilter";

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
  frameAssets: FrameAssetOption[];
  stats: PetStats | null;
  foods: FoodItem[];
  settings: AppSettings | null;
  sessions: ChatSession[];
  messages: ChatMessage[];
  currentState: RuntimeAnimationState;
  bubble: string | null;

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
  streamingContent: string;
  currentRequestId: string | null;
  currentSessionId: string | null;
  pendingChatMessage: string | null;

  // Box button state
  boxPosition: { x: number; y: number } | null;
  boxOpen: boolean;
  toolboxVisible: boolean;

  // Interaction tracking
  rapidClickCount: number;
  rapidClickFirstAt: number;
  rapidCancelCount: number;
  rapidCancelFirstAt: number;
  lastInteractionAt: number;
  hasTriggered5MinIdle: boolean;
  hasTriggered10MinSleep: boolean;
  behaviorFlow: BehaviorFlow | null;
  sleepIntentQueued: boolean;

  bootstrap: () => Promise<void>;
  openPanel: (panel: ActivePanel) => void;
  closePanel: () => void;
  sendMessage: (content: string) => Promise<void>;
  feed: (foodId: string) => Promise<FeedResult | null>;
  replaceFood: (request: FoodReplaceRequest) => Promise<void>;
  reorderFoods: (foodIds: string[]) => Promise<void>;
  createCharacter: (request: CreateCharacterRequest) => Promise<void>;
  updateCharacter: (request: UpdateCharacterRequest) => Promise<void>;
  previewCharacterScale: (characterId: string, displayScale: number) => void;
  updateCharacterScale: (characterId: string, displayScale: number) => Promise<void>;
  switchCharacter: (characterId: string) => Promise<void>;
  deleteCharacter: (characterId: string) => Promise<void>;
  deleteChatSession: (sessionId: string) => Promise<void>;
  deleteChatHistoryDay: (dateKey: string) => Promise<void>;
  loadFrameAssets: () => Promise<void>;
  chooseAndImportFrameAsset: () => Promise<FrameAssetOption | null>;
  importFrameAssetFromPath: (path: string) => Promise<FrameAssetOption>;
  deleteFrameAsset: (assetId: string) => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
  resetStats: () => Promise<void>;
  validateSkinPath: (path: string) => Promise<SkinValidationReport>;
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
  sendAiChat: (content: string) => Promise<void>;
  abortAiRequest: () => Promise<void>;
  trackRapidClick: () => void;
  trackRapidCancel: () => void;
  recordPetInteraction: () => Promise<void>;
  lullToSleep: () => void;
}

let typewriterTimer: ReturnType<typeof setTimeout> | null = null;
let typewriterQueue = "";
let streamDisplayBuffer = "";

const CHAT_ACTIONS = new Set(["calm", "happy", "sad", "angry", "comfort", "cheer_up"]);
const ACTIVE_ACTION_KEYS = new Set(["eat_food", "wake_up", "sleeping", "run_left", "run_right", "fall_down", "fly_up", "sit", "sit_down"]);
const PHYSICAL_STATE_KEYS = new Set([FALL_KEY, "dizzy", "fly_up"]);
const INTERACTION_STATE_KEYS = new Set(["thinking", "eat_food", "happy", "sad", "angry", "comfort", "cheer_up", "error", WAKE_UP_KEY]);
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

function wakeFromSleepUpdates(now = Date.now()): Partial<AppStoreState> {
  return {
    ...interactionReset(now),
    behaviorFlow: "waking",
    sleepIntentQueued: false,
    currentState: createRuntimeState(WAKE_UP_KEY, now),
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

function enqueueVisibleStreamChunk(
  set: (partial: Partial<AppStoreState> | ((state: AppStoreState) => Partial<AppStoreState>)) => void,
  content: string
): void {
  streamDisplayBuffer += content;
  const partition = partitionStreamBuffer(streamDisplayBuffer);
  streamDisplayBuffer = partition.pending;
  if (partition.visible) enqueueTypewriter(set, partition.visible);
}

function applyBootstrap(set: (partial: Partial<AppStoreState>) => void, payload: BootstrapPayload): void {
  set({
    loading: false,
    dataPaths: payload.dataPaths,
    character: payload.character,
    characters: payload.characters,
    stats: payload.stats,
    foods: payload.foods,
    settings: payload.settings,
    sessions: payload.sessions,
    messages: payload.messages,
    error: null
  });
}

// Default bounds before screen info is loaded
const DEFAULT_BOUNDS: WorldBounds = createBounds(1920, 1080, 60);

export const useAppStore = create<AppStoreState>((set, get) => ({
  loading: true,
  error: null,
  activePanel: null,
  dataPaths: null,
  character: null,
  characters: [],
  frameAssets: [{ id: "rick_default", name: "系统默认 Rick", path: null, builtIn: true, importedAt: null, shortActionKeys: DEFAULT_SHORT_ACTION_KEYS }],
  stats: null,
  foods: [],
  settings: null,
  sessions: [],
  messages: [],
  currentState: createRuntimeState("calm"),
  bubble: null,

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
  streamingContent: "",
  currentRequestId: null,
  currentSessionId: null,
  pendingChatMessage: null,

  // Box button defaults (null = will be positioned on first render)
  boxPosition: null,
  boxOpen: false,
  toolboxVisible: true,

  // Interaction tracking defaults
  rapidClickCount: 0,
  rapidClickFirstAt: 0,
  rapidCancelCount: 0,
  rapidCancelFirstAt: 0,
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
      const startX = freshBounds.left + Math.random() * (freshBounds.right - freshBounds.left - PET_WIDTH) + PET_WIDTH / 2;
      const startY = freshBounds.top;

      set({
        currentState: createRuntimeState("calm"),
        bubble: "Rick 已上线",
        body: createBody(startX, startY, PET_WIDTH, PET_HEIGHT),
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
      const bounds = createBounds(info.screen_width, info.screen_height, info.taskbar_height);
      set({ screenInfo: info, bounds });
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
      set({ screenInfo: info, bounds });
    }
  },

  updatePhysics: (dt: number) => {
    const state = get();

    if (state.onGround && state.movementState.type === "idle" && !state.throwTween) {
      return;
    }

    // Handle throw tween
    if (state.throwTween) {
      const tween = state.throwTween;
      const elapsed = Date.now() - tween.startTime;
      const progress = Math.min(elapsed / THROW_DURATION, 1);
      // QuartEaseOut
      const ease = 1 - Math.pow(1 - progress, 4);

      const newX = tween.startX + (tween.targetX - tween.startX) * ease;
      const newY = tween.startY + (tween.targetY - tween.startY) * ease;

      if (progress >= 1) {
        // Tween complete, enable physics for falling
        set({
          throwTween: null,
          body: {
            ...state.body,
            x: tween.targetX,
            y: tween.targetY,
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
        body: { ...state.body, x: newX, y: newY, enabled: false },
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

        // fall_down has 4 frames at 14fps; switch to dizzy after it finishes.
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
          // tickAnimation returns dizzy to calm after its configured duration.
          // tickMovement can resume autonomous movement after dizzy expires.
        }
        // 落地后自动发送排队消息
        if (cur.pendingChatMessage && cur.onGround) {
          const msg = cur.pendingChatMessage;
          set({ pendingChatMessage: null });
          get().sendAiChat(msg).catch(() => {});
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
      body: { ...state.body, x, y, enabled: false },
    }));
  },

  endDrag: (velocityX: number, velocityY: number) => {
    const state = get();
    const body = state.body;

    // Clamp throw to not go above screen top
    const targetX = body.x + velocityX * TWEEN_ACCELERATION;
    const targetY = Math.max(0, body.y + velocityY * TWEEN_ACCELERATION);

    set({
      throwTween: {
        startX: body.x,
        startY: body.y,
        targetX,
        targetY,
        startTime: Date.now(),
      },
      movementState: { ...state.movementState, type: "thrown" },
      currentState: createRuntimeState("fly_up"),
      behaviorFlow: null,
      sleepIntentQueued: false,
    });
    void get().recordPetInteraction();
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
      // Stop moving and play calm.
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

  sendMessage: async (content) => {
    if (!content.trim()) {
      return;
    }

    set(interactionReset());
    const sessionId = get().sessions[0]?.id ?? null;
    set((state) => ({ currentState: transitionTo(state.currentState, "thinking") }));

    try {
      const result = await sendChatMessage(content.trim(), sessionId);
      set((state) => ({
        sessions: state.sessions.some((session) => session.id === result.session.id) ? state.sessions : [result.session, ...state.sessions],
        messages: [...state.messages, result.userMessage, result.assistantMessage],
        stats: result.stats,
        foods: result.foods,
        currentState: createRuntimeState(result.triggeredStateKey)
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : "聊天发送失败",
        currentState: createRuntimeState("error"),
        bubble: "AI 请求失败"
      });
    }
  },

  feed: async (foodId) => {
    const wakingFromSleep = get().currentState.definition.key === SLEEPING_KEY || get().behaviorFlow === "waking";
    if (wakingFromSleep) {
      set((state) => ({
        ...wakeFromSleepUpdates(),
        movementState: { ...state.movementState, type: "idle", animationKey: WAKE_UP_KEY, idleTimer: 999999 },
        body: { ...state.body, vx: 0 },
      }));
    } else {
      set(interactionReset());
    }

    try {
      const result = await feedFood(foodId);
      // Map feed state keys to valid animation keys
      const feedAnimMap: Record<string, string> = {
        feed_neutral: "eat_food",
        feed_blocked: "angry",
      };
      const animKey = feedAnimMap[result.triggeredStateKey] ?? "eat_food";
      set((state) => ({
        foods: state.foods.map((food) => (food.id === result.food.id ? result.food : food)),
        stats: result.stats,
        currentState: state.behaviorFlow === "waking" ? state.currentState : createRuntimeState(animKey),
        bubble: result.message
      }));
      return result;
    } catch (error) {
      set((_state) => ({
        error: error instanceof Error ? error.message : "投喂失败",
        currentState: createRuntimeState("angry"),
        bubble: "投喂失败"
      }));
      return null;
    }
  },

  replaceFood: async (request) => {
    const updated = await replaceFood(request);
    set((state) => ({
      foods: state.foods.map((food) => (food.id === updated.id ? updated : food)),
      bubble: "Food slot updated",
    }));
  },

  reorderFoods: async (foodIds) => {
    const foods = await reorderFoods(foodIds);
    set({ foods, bubble: "Food order saved" });
  },

  createCharacter: async (request) => {
    applyBootstrap(set, await createCharacter(request));
    set({ currentState: createRuntimeState("calm"), bubble: "人物已创建" });
  },

  updateCharacter: async (request) => {
    applyBootstrap(set, await updateCharacter(request));
    set({ currentState: createRuntimeState("calm"), bubble: "人物已更新" });
  },

  previewCharacterScale: (characterId, displayScale) => {
    const scale = Math.min(3, Math.max(0.75, Number.isFinite(displayScale) ? displayScale : 1));
    set((state) => ({
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
    applyBootstrap(set, await updateCharacterScaleApi(characterId, scale));
  },

  switchCharacter: async (characterId) => {
    applyBootstrap(set, await switchCharacter(characterId));
    set({ currentState: createRuntimeState("calm"), bubble: "已切换人物" });
  },

  deleteCharacter: async (characterId) => {
    applyBootstrap(set, await deleteCharacter(characterId));
    set({ currentState: createRuntimeState("calm"), bubble: "人物已删除" });
  },

  deleteChatSession: async (sessionId) => {
    applyBootstrap(set, await deleteChatSession(sessionId));
  },

  deleteChatHistoryDay: async (dateKey) => {
    applyBootstrap(set, await deleteChatHistoryDay(dateKey));
  },

  saveSettings: async (settings) => {
    const saved = await saveSettings(settings);
    set({ settings: saved, bubble: "设置已保存" });
  },

  resetStats: async () => {
    applyBootstrap(set, await resetStats());
    set({ currentState: createRuntimeState("calm"), bubble: "能量与好感已重置" });
  },

  validateSkinPath: (path) => validateSkinPath(path),

  loadFrameAssets: async () => {
    set({ frameAssets: await listFrameAssets() });
  },

  chooseAndImportFrameAsset: async () => {
    const asset = await chooseAndImportFrameAsset();
    if (asset) {
      const frameAssets = await listFrameAssets();
      set({
        frameAssets: mergeFrameAsset(frameAssets, asset),
        bubble: "序列帧资源已导入",
      });
    }
    return asset;
  },

  importFrameAssetFromPath: async (path) => {
    const asset = await importFrameAssetFromPath(path);
    const frameAssets = await listFrameAssets();
    set({
      frameAssets: mergeFrameAsset(frameAssets, asset),
      bubble: "序列帧资源已导入",
    });
    return asset;
  },

  deleteFrameAsset: async (assetId) => {
    const frameAssets = await deleteFrameAssetApi(assetId);
    set({ frameAssets, bubble: "素材包已删除" });
  },

  openDataDir: () => openDataDir(),

  tickAnimation: () => {
    const state = get();
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
    if (!content.trim()) return;

    const state = get();
    set(interactionReset());

    // Queue the message until physical fly/fall/dizzy motion finishes.
    const mType = state.movementState.type;
    if (
      !state.onGround ||
      mType === "thrown" ||
      mType === "fall" ||
      (state.currentState.definition.key === "fly_up") ||
      (state.currentState.definition.key === "fall_down") ||
      (state.currentState.definition.key === "dizzy")
    ) {
      set({ pendingChatMessage: content.trim() });
      return;
    }

    const requestId = crypto.randomUUID();
    const sessionId = state.sessions[0]?.id ?? crypto.randomUUID();
    const wakingFromSleep = state.currentState.definition.key === SLEEPING_KEY || state.behaviorFlow === "waking";

    // Start thinking unless sleeping needs the wake_up transition first.
    set({
      ...interactionReset(),
      chatStreaming: true,
      streamingContent: "",
      currentRequestId: requestId,
      currentSessionId: sessionId,
      currentState: wakingFromSleep ? createRuntimeState(WAKE_UP_KEY) : createRuntimeState("thinking"),
      behaviorFlow: wakingFromSleep ? "waking" : state.behaviorFlow,
      movementState: wakingFromSleep ? { ...state.movementState, type: "idle", animationKey: WAKE_UP_KEY, idleTimer: 999999 } : state.movementState,
    });

    // Add user message to local messages immediately
    const now = new Date().toISOString();
    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      sessionId,
      characterId: state.character?.id ?? "",
      role: "user",
      content: content.trim(),
      createdAt: now,
      tokenCount: content.length,
      metadataJson: null,
    };
    set((s) => ({ messages: [...s.messages, userMsg] }));

    const unlisten = listen<AiChatEvent>("ai-chat-event", (event) => {
      const evt = event.payload;
      if (evt.request_id !== requestId) return;

      switch (evt.event) {
        case "start":
          // Already in thinking state
          break;

        case "delta":
          enqueueVisibleStreamChunk(set, evt.content ?? "");
          break;

        case "done": {
          clearTypewriter();
          const fullResponse = evt.full_response ?? get().streamingContent;
          const animKey = normalizeChatAction(evt.action);
          set({
            chatStreaming: false,
            streamingContent: "",
            currentRequestId: null,
            currentSessionId: null,
          });

          // Add assistant message to local messages
          const assistantMsg: ChatMessage = {
            id: crypto.randomUUID(),
            sessionId,
            characterId: state.character?.id ?? "",
            role: "assistant",
            content: fullResponse,
            createdAt: new Date().toISOString(),
            tokenCount: 0,
            metadataJson: JSON.stringify({
              action: animKey,
              rawAction: evt.action ?? null,
            }),
          };
          set((s) => ({
            messages: [...s.messages, assistantMsg],
            currentState: canPlayChatAction(s) ? createRuntimeState(animKey) : s.currentState,
          }));

          unlisten.then((fn) => fn());
          break;
        }

        case "error": {
          clearTypewriter();
          set({
            chatStreaming: false,
            streamingContent: "",
            currentRequestId: null,
            currentSessionId: null,
            currentState: createRuntimeState("error"),
            bubble: evt.message ?? "AI 请求失败",
          });
          unlisten.then((fn) => fn());
          break;
        }

        case "aborted": {
          clearTypewriter();
          set({
            chatStreaming: false,
            currentRequestId: null,
            currentSessionId: null,
            currentState: createRuntimeState("calm"),
          });
          unlisten.then((fn) => fn());
          break;
        }
      }
    });

    // Start streaming; progress arrives through the event listener.
    try {
      await streamAiMessage(requestId, sessionId, content.trim());
      applyBootstrap(set, await bootstrapApp());
    } catch (err) {
      // If invoke itself fails (not the stream), clean up
      set({
        chatStreaming: false,
        streamingContent: "",
        currentRequestId: null,
        currentState: createRuntimeState("error"),
        bubble: err instanceof Error ? err.message : "AI 请求失败",
      });
      unlisten.then((fn) => fn());
    }
  },

  abortAiRequest: async () => {
    const state = get();
    const requestId = state.currentRequestId;
    if (!requestId) return;
    const sessionId = state.currentSessionId ?? state.sessions[0]?.id ?? "";
    const partial = state.streamingContent;

    // Track rapid cancels for interaction angry
    get().trackRapidCancel();
    clearTypewriter();

    try {
      await abortAiRequestApi(requestId);
    } catch {
      // Request may have already completed
    }

    const now = new Date().toISOString();
    const assistantMsg: ChatMessage | null = partial.trim()
      ? {
          id: crypto.randomUUID(),
          sessionId,
          characterId: state.character?.id ?? "",
          role: "assistant",
          content: `${partial}\n\n[已中断]`,
          createdAt: now,
          tokenCount: partial.length,
          metadataJson: JSON.stringify({ status: "interrupted" }),
        }
      : null;

    set((s) => ({
      chatStreaming: false,
      streamingContent: "",
      currentRequestId: null,
      currentSessionId: null,
      currentState: createRuntimeState("calm"),
      messages: assistantMsg ? [...s.messages, assistantMsg] : s.messages,
    }));

    if (sessionId && partial.trim()) {
      try {
        applyBootstrap(set, await saveInterruptedAiMessage(requestId, sessionId, partial));
      } catch {
        // Local interrupted message is already preserved.
      }
    }
  },

  trackRapidClick: () => {
    const now = Date.now();
    const state = get();
    if (now - state.rapidClickFirstAt > 10000) {
      set({ rapidClickCount: 1, rapidClickFirstAt: now });
    } else {
      set({ rapidClickCount: state.rapidClickCount + 1 });
    }
  },

  trackRapidCancel: () => {
    const now = Date.now();
    const state = get();
    if (now - state.rapidCancelFirstAt > 30000) {
      set({ rapidCancelCount: 1, rapidCancelFirstAt: now });
    } else {
      set({ rapidCancelCount: state.rapidCancelCount + 1 });
    }
  },

  recordPetInteraction: async () => {
    set(interactionReset());
    try {
      applyBootstrap(set, await recordPetInteraction());
    } catch {
      // Interaction stats are best-effort; movement should not be blocked.
    }
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
