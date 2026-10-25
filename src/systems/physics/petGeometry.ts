import { PET_WIDTH } from "../movement/constants";
import { checkAndClampBounds, type PhysicsBody, type WorldBounds } from "./engine";

export function petSize(scale: number, bounds: WorldBounds): number {
  const safe = Number.isFinite(scale) ? Math.min(3, Math.max(0.75, scale)) : 1;
  return Math.max(1, Math.min(Math.round(PET_WIDTH * safe), bounds.right - bounds.left, bounds.bottom - bounds.top));
}

export function clampPetPosition(body: PhysicsBody, bounds: WorldBounds, x = body.x, y = body.y): PhysicsBody {
  const clamped = { ...body, x, y, enabled: true };
  checkAndClampBounds(clamped, bounds);
  return { ...clamped, enabled: body.enabled };
}

/** Keep the feet anchored when changing size, then fit the entire sprite on screen. */
export function resizePetBody(body: PhysicsBody, bounds: WorldBounds, scale: number): PhysicsBody {
  const size = petSize(scale, bounds);
  return clampPetPosition({ ...body, width: size, height: size, y: body.y + body.height - size }, bounds);
}
