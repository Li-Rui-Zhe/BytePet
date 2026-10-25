import { describe, expect, it } from "vitest";
import { createBounds, PET_VISUAL_GROUND_OFFSET } from "../systems/physics/engine";

describe("physics bounds", () => {
  it("places the sprite box on the taskbar edge when frames have no bottom padding", () => {
    const bounds = createBounds(1920, 1080, 40);

    expect(bounds.bottom).toBe(1080 - 40 + PET_VISUAL_GROUND_OFFSET);
  });

  it("does not place the pet below the physical screen when the taskbar is hidden", () => {
    const bounds = createBounds(1920, 1080, 0);

    expect(bounds.bottom).toBe(1080);
  });

  it("respects a top menu bar and a side Dock when an explicit work area is provided", () => {
    expect(createBounds(1440, 900, 0, { left: 65, top: 25, right: 1440, bottom: 900 })).toEqual({ left: 65, top: 25, right: 1440, bottom: 900 });
  });

  it("uses window-relative Retina work-area dimensions without scaling twice", () => {
    expect(createBounds(1440, 810, 0, { left: 0, top: 0, right: 1440, bottom: 810 })).toEqual({ left: 0, top: 0, right: 1440, bottom: 810 });
  });

  it("clamps invalid work areas or falls back to the legacy calculation", () => {
    expect(createBounds(100, 100, 10, { left: -10, top: -20, right: 130, bottom: 120 })).toEqual({ left: 0, top: 0, right: 100, bottom: 100 });
    expect(createBounds(100, 100, 10, { left: 50, top: 0, right: 40, bottom: 90 }).bottom).toBe(90);
  });
});
