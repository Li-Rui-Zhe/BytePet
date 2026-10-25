// Physics constants (inspired by WindowPet Arcade Physics)
export const GRAVITY = 200; // pixels/s²
export const MAX_FALL_SPEED = 600; // pixels/s
export const WALK_SPEED = 54; // pixels/s (FRAME_RATE * 6 from WindowPet)
export const PET_WIDTH = 152; // sprite display width in px
export const PET_HEIGHT = 152; // sprite display height in px

// Project sampled pixel/second velocity into a short, screen-clamped throw.
export const THROW_PROJECTION_SECONDS = 0.18;
export const THROW_DURATION = 600; // ms

// Behavior timing
export const MOVEMENT_TICK_MS = 111; // ~9 FPS decision rate (WindowPet UPDATE_DELAY)
export const IDLE_MIN_DURATION = 3000; // ms
export const IDLE_MAX_DURATION = 6000; // ms
export const LIGHT_IDLE_MS = 5 * 60 * 1000;
export const SLEEP_IDLE_MS = 10 * 60 * 1000;
export const SIT_DOWN_DURATION_MS = 8 * 1000;
export const SHORT_ACTION_DURATION_MS = 8 * 1000;

// Probability constants
export const WALK_TO_IDLE_CHANCE = 28 / 1000; // walking stops in a few seconds on average
export const FLIP_CHANCE = 3 / 2000; // ~0.15% per tick
export const IDLE_TO_WALK_CHANCE = 0.2; // evaluated only when an idle pause expires
export const IDLE_TO_SIT_CHANCE = 0.15; // evaluated only when an idle pause expires
export const IDLE_TO_CALM_CHANCE = 0.15; // evaluated only when an idle pause expires
export const IDLE_TO_SHORT_ACTION_CHANCE = 0.5; // evaluated only when an idle pause expires
export const POST_MOVE_SIT_CHANCE = 0.4;


// Available walk animation keys
export const WALK_LEFT_KEY = "run_left";
export const WALK_RIGHT_KEY = "run_right";
export const FALL_KEY = "fall_down";
export const IDLE_KEY = "calm";
export const YAWN_KEY = "yawn";
export const SIT_KEY = "sit";
export const SIT_DOWN_KEY = "sit_down";
export const SLEEPING_KEY = "sleeping";
export const WAKE_UP_KEY = "wake_up";
