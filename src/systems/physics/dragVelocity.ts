export interface DragSample { x: number; y: number; time: number }
const WINDOW_MS = 100;
const MAX_SPEED = 1800;

export function recordDragSample(samples: DragSample[], sample: DragSample): DragSample[] {
  return [...samples.filter(item => sample.time - item.time <= WINDOW_MS), sample].slice(-24);
}

export function releaseVelocity(samples: DragSample[], release: DragSample): { x: number; y: number } {
  const last = samples.at(-1);
  if (!last || release.time - last.time > WINDOW_MS) return { x: 0, y: 0 };
  const recent = recordDragSample(samples, release);
  const first = recent.find(item => release.time > item.time);
  if (!first) return { x: 0, y: 0 };
  const dt = (release.time - first.time) / 1000;
  const x = (release.x - first.x) / dt;
  const y = (release.y - first.y) / dt;
  const speed = Math.hypot(x, y);
  if (!Number.isFinite(speed)) return { x: 0, y: 0 };
  const factor = speed > MAX_SPEED ? MAX_SPEED / speed : 1;
  return { x: x * factor, y: y * factor };
}
