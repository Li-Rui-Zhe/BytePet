import { afterEach, describe, expect, it, vi } from "vitest";
import { recordDragSample, releaseVelocity } from "../systems/physics/dragVelocity";
import { clampPetPosition, petSize, resizePetBody } from "../systems/physics/petGeometry";
import { createBody, createBounds } from "../systems/physics/engine";
import { createScaleSaver } from "../systems/physics/scaleSaver";

afterEach(() => vi.useRealTimers());

describe("release velocity", () => {
  it("keeps a fast swipe when mouseup has the same position as the last move", () => {
    const points = [{ x: 0, y: 0, time: 0 }, { x: 100, y: 0, time: 16 }, { x: 200, y: 0, time: 32 }];
    const velocity = releaseVelocity(points, { x: 200, y: 0, time: 33 });
    expect(velocity.x).toBeGreaterThan(0);
    expect(Math.hypot(velocity.x, velocity.y)).toBeLessThanOrEqual(1800);
  });
  it("drops rather than throws after holding still", () => {
    expect(releaseVelocity([{ x: 0, y: 0, time: 0 }, { x: 100, y: 0, time: 16 }], { x: 100, y: 0, time: 200 })).toEqual({ x: 0, y: 0 });
  });
  it("handles zero time, stationary samples and a recent direction reversal", () => {
    expect(releaseVelocity([{ x: 10, y: 0, time: 1 }], { x: 10, y: 0, time: 1 })).toEqual({ x: 0, y: 0 });
    expect(releaseVelocity([{ x: 10, y: 0, time: 0 }], { x: 10, y: 0, time: 16 })).toEqual({ x: 0, y: 0 });
    const samples = recordDragSample([{ x: 0, y: 0, time: 0 }, { x: 200, y: 0, time: 150 }], { x: 100, y: 0, time: 180 });
    expect(releaseVelocity(samples, { x: 90, y: 0, time: 190 }).x).toBeLessThan(0);
  });
});

describe("scaled geometry", () => {
  const bounds = createBounds(1920, 1080, 60);
  it("anchors feet and uses the same scaled size for both axes", () => {
    const body = createBody(700, 868, 152, 152);
    const resized = resizePetBody(body, bounds, 3);
    expect(resized.width).toBe(456);
    expect(resized.height).toBe(456);
    expect(resized.y + resized.height).toBe(1020);
  });
  it("fits left, right, top and bottom even while dragging", () => {
    const body = { ...createBody(76, 0, 152, 152), enabled: false };
    const resized = resizePetBody(body, bounds, 3);
    expect(resized.x - resized.width / 2).toBeGreaterThanOrEqual(0);
    expect(resized.y).toBeGreaterThanOrEqual(0);
    const dragged = clampPetPosition(resized, bounds, 100000, 100000);
    expect(dragged.x + dragged.width / 2).toBeLessThanOrEqual(bounds.right);
    expect(dragged.y + dragged.height).toBeLessThanOrEqual(bounds.bottom);
    expect(dragged.enabled).toBe(false);
  });
  it("fits smaller work areas and rejects invalid scales", () => {
    expect(petSize(3, createBounds(300, 250, 20))).toBe(230);
    expect(petSize(Number.NaN, bounds)).toBe(152);
  });
});

describe("scale save scheduling", () => {
  it("flushes once on release/blur without a duplicate delayed write", async () => {
    vi.useFakeTimers();
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createScaleSaver(save, vi.fn());
    saver.schedule("a", 1.1);
    saver.schedule("a", 1.5);
    saver.flush("a");
    saver.flush("a");
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledExactlyOnceWith("a", 1.5);
  });
  it("flushes unsaved keyboard changes when a panel closes", async () => {
    vi.useFakeTimers();
    const save = vi.fn().mockResolvedValue(undefined);
    const saver = createScaleSaver(save, vi.fn());
    saver.schedule("a", 1.8);
    saver.flush();
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledExactlyOnceWith("a", 1.8);
  });
  it("serializes writes and can retry after failure", async () => {
    vi.useFakeTimers();
    let finish!: () => void;
    const save = vi.fn().mockImplementationOnce(() => new Promise<void>(resolve => { finish = resolve; })).mockRejectedValueOnce(new Error("failed")).mockResolvedValue(undefined);
    const error = vi.fn();
    const saver = createScaleSaver(save, error);
    saver.schedule("a", 1.1); saver.flush();
    await Promise.resolve();
    saver.schedule("a", 1.9); saver.flush();
    expect(save).toHaveBeenCalledTimes(1);
    finish();
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledTimes(2);
    expect(error).toHaveBeenCalledTimes(1);
    saver.schedule("a", 1.9); saver.flush();
    await vi.runAllTimersAsync();
    expect(save).toHaveBeenCalledTimes(3);
  });
});
