import { GRAVITY, MAX_FALL_SPEED } from "../movement/constants";

export interface PhysicsBody {
  x: number;
  y: number;
  vx: number;
  vy: number;
  width: number;
  height: number;
  allowGravity: boolean;
  enabled: boolean;
}

export interface WorldBounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface CollisionEvent {
  up: boolean;
  down: boolean;
  left: boolean;
  right: boolean;
}

export const PET_VISUAL_GROUND_OFFSET = 0;

const NO_COLLISION: CollisionEvent = { up: false, down: false, left: false, right: false };

export function applyGravity(body: PhysicsBody, dt: number): void {
  if (!body.allowGravity || !body.enabled) return;
  body.vy += GRAVITY * dt;
  if (body.vy > MAX_FALL_SPEED) {
    body.vy = MAX_FALL_SPEED;
  }
}

export function updatePosition(body: PhysicsBody, dt: number): void {
  if (!body.enabled) return;
  body.x += body.vx * dt;
  body.y += body.vy * dt;
}

export function checkAndClampBounds(body: PhysicsBody, bounds: WorldBounds): CollisionEvent {
  if (!body.enabled) return NO_COLLISION;

  const event: CollisionEvent = { up: false, down: false, left: false, right: false };

  const halfW = body.width / 2;
  const bodyLeft = body.x - halfW;
  const bodyRight = body.x + halfW;
  const bodyTop = body.y;
  const bodyBottom = body.y + body.height;

  if (bodyLeft < bounds.left) {
    body.x = bounds.left + halfW;
    body.vx = 0;
    event.left = true;
  }
  if (bodyRight > bounds.right) {
    body.x = bounds.right - halfW;
    body.vx = 0;
    event.right = true;
  }
  if (bodyTop < bounds.top) {
    body.y = bounds.top;
    body.vy = 0;
    event.up = true;
  }
  if (bodyBottom > bounds.bottom) {
    body.y = bounds.bottom - body.height;
    body.vy = 0;
    event.down = true;
  }

  return event;
}

export function updatePhysics(
  body: PhysicsBody,
  dt: number,
  bounds: WorldBounds
): CollisionEvent {
  if (!body.enabled) return NO_COLLISION;

  applyGravity(body, dt);
  updatePosition(body, dt);
  return checkAndClampBounds(body, bounds);
}

export function createBody(x: number, y: number, width: number, height: number): PhysicsBody {
  return {
    x,
    y,
    vx: 0,
    vy: 0,
    width,
    height,
    allowGravity: true,
    enabled: true,
  };
}

export function createBounds(screenWidth: number, screenHeight: number, taskbarHeight: number, workArea?: WorldBounds): WorldBounds {
  if (workArea && Object.values(workArea).every(Number.isFinite)) {
    const area = { left: Math.max(0, workArea.left), top: Math.max(0, workArea.top), right: Math.min(screenWidth, workArea.right), bottom: Math.min(screenHeight, workArea.bottom) };
    if (area.right > area.left && area.bottom > area.top) return area;
  }
  const workAreaBottom = screenHeight - taskbarHeight;

  return {
    left: 0,
    top: 0,
    right: screenWidth,
    bottom: Math.min(screenHeight, workAreaBottom + PET_VISUAL_GROUND_OFFSET),
  };
}
