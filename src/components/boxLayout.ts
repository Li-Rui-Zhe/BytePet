import { useEffect, useState } from "react";

export const BOX_BUTTON_SIZE = 80;
export const BOX_SAFE_MARGIN = 14;
export const FLOATING_GAP = 12;
export const TOOLBOX_WIDTH = 228;
export const TOOLBOX_HEIGHT = 198;
export const PANEL_WIDTH = 330;
export const PANEL_HEIGHT = 372;

export type FloatingPlacement = "top" | "right" | "bottom" | "left";

export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

interface Rect extends Point, Size {}

interface FloatingLayout extends Point, Size {
  placement: FloatingPlacement;
}

export function clampValue(value: number, min: number, max: number): number {
  if (max < min) return min;
  return Math.max(min, Math.min(value, max));
}

export function getViewportSize(): Size {
  if (typeof window === "undefined") {
    return { width: 800, height: 600 };
  }

  return {
    width: window.innerWidth,
    height: window.innerHeight,
  };
}

export function useViewportSize(): Size {
  const [viewport, setViewport] = useState(getViewportSize);

  useEffect(() => {
    const update = () => setViewport(getViewportSize());
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, []);

  return viewport;
}

export function getDefaultBoxPosition(viewport = getViewportSize()): Point {
  return {
    x: viewport.width - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN,
    y: viewport.height - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN,
  };
}

export function clampBoxPosition(x: number, y: number, viewport = getViewportSize()): Point {
  return {
    x: clampValue(x, BOX_SAFE_MARGIN, viewport.width - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN),
    y: clampValue(y, BOX_SAFE_MARGIN, viewport.height - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN),
  };
}

function toAnchorRect(boxPosition: Point): Rect {
  return {
    x: boxPosition.x,
    y: boxPosition.y,
    width: BOX_BUTTON_SIZE,
    height: BOX_BUTTON_SIZE,
  };
}

function getIntersectionArea(a: Rect, b: Rect): number {
  const x = Math.max(0, Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x));
  const y = Math.max(0, Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y));
  return x * y;
}

function getOverflowScore(rect: Rect, viewport: Size): number {
  const left = Math.max(0, BOX_SAFE_MARGIN - rect.x);
  const top = Math.max(0, BOX_SAFE_MARGIN - rect.y);
  const right = Math.max(0, rect.x + rect.width - viewport.width + BOX_SAFE_MARGIN);
  const bottom = Math.max(0, rect.y + rect.height - viewport.height + BOX_SAFE_MARGIN);
  return left + top + right + bottom;
}

function clampFloatingRect(rect: Rect, viewport: Size): Rect {
  return {
    ...rect,
    x: clampValue(rect.x, BOX_SAFE_MARGIN, viewport.width - rect.width - BOX_SAFE_MARGIN),
    y: clampValue(rect.y, BOX_SAFE_MARGIN, viewport.height - rect.height - BOX_SAFE_MARGIN),
  };
}

function getFloatingCandidate(anchor: Rect, size: Size, placement: FloatingPlacement, gap: number): Rect {
  switch (placement) {
    case "right":
      return {
        x: anchor.x + anchor.width + gap,
        y: anchor.y + anchor.height / 2 - size.height / 2,
        ...size,
      };
    case "bottom":
      return {
        x: anchor.x + anchor.width / 2 - size.width / 2,
        y: anchor.y + anchor.height + gap,
        ...size,
      };
    case "left":
      return {
        x: anchor.x - size.width - gap,
        y: anchor.y + anchor.height / 2 - size.height / 2,
        ...size,
      };
    case "top":
    default:
      return {
        x: anchor.x + anchor.width / 2 - size.width / 2,
        y: anchor.y - size.height - gap,
        ...size,
      };
  }
}

export function getToolboxLayout(boxPosition: Point | null, viewport: Size): FloatingLayout {
  const size = {
    width: Math.min(TOOLBOX_WIDTH, viewport.width - BOX_SAFE_MARGIN * 2),
    height: Math.min(TOOLBOX_HEIGHT, viewport.height - BOX_SAFE_MARGIN * 2),
  };

  if (!boxPosition) {
    return {
      x: viewport.width - size.width - BOX_SAFE_MARGIN,
      y: viewport.height - size.height - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN - FLOATING_GAP,
      placement: "top",
      ...size,
    };
  }

  const anchor = toAnchorRect(boxPosition);
  const placements: FloatingPlacement[] = ["top", "left", "right", "bottom"];
  const scored = placements.map((placement, index) => {
    const raw = getFloatingCandidate(anchor, size, placement, FLOATING_GAP);
    const clamped = clampFloatingRect(raw, viewport);
    const overlap = getIntersectionArea(clamped, anchor);
    const overflow = getOverflowScore(raw, viewport);

    return {
      rect: clamped,
      placement,
      score: overflow * 120 + overlap * 4 + index,
    };
  });

  const best = scored.sort((a, b) => a.score - b.score)[0];
  return {
    x: best.rect.x,
    y: best.rect.y,
    width: best.rect.width,
    height: best.rect.height,
    placement: best.placement,
  };
}

export function getPanelLayout(boxPosition: Point | null, viewport: Size, customSize?: Size | null): FloatingLayout {
  const baseWidth = clampValue(customSize?.width ?? PANEL_WIDTH, 286, viewport.width - BOX_SAFE_MARGIN * 2);
  const baseHeight = clampValue(customSize?.height ?? PANEL_HEIGHT, 286, viewport.height - BOX_SAFE_MARGIN * 2);

  if (!boxPosition) {
    const rect = clampFloatingRect({
      x: viewport.width - baseWidth - BOX_SAFE_MARGIN,
      y: viewport.height - baseHeight - BOX_BUTTON_SIZE - BOX_SAFE_MARGIN - FLOATING_GAP,
      width: baseWidth,
      height: baseHeight,
    }, viewport);
    return {
      ...rect,
      placement: "top",
    };
  }

  const anchor = toAnchorRect(boxPosition);
  const above = anchor.y - FLOATING_GAP - BOX_SAFE_MARGIN;
  const below = viewport.height - (anchor.y + anchor.height) - FLOATING_GAP - BOX_SAFE_MARGIN;
  const left = anchor.x - FLOATING_GAP - BOX_SAFE_MARGIN;
  const right = viewport.width - (anchor.x + anchor.width) - FLOATING_GAP - BOX_SAFE_MARGIN;

  const sizes: Record<FloatingPlacement, Size> = {
    top: { width: baseWidth, height: Math.min(baseHeight, Math.max(260, above)) },
    bottom: { width: baseWidth, height: Math.min(baseHeight, Math.max(260, below)) },
    left: { width: Math.min(baseWidth, Math.max(286, left)), height: baseHeight },
    right: { width: Math.min(baseWidth, Math.max(286, right)), height: baseHeight },
  };

  const placements: FloatingPlacement[] = ["top", "left", "right", "bottom"];
  const scored = placements.map((placement, index) => {
    const size = sizes[placement];
    const raw = getFloatingCandidate(anchor, size, placement, FLOATING_GAP);
    const clamped = clampFloatingRect(raw, viewport);
    const overlap = getIntersectionArea(clamped, anchor);
    const overflow = getOverflowScore(raw, viewport);
    const cramped = Math.max(0, 300 - size.height) + Math.max(0, 300 - size.width);

    return {
      rect: clamped,
      placement,
      score: overflow * 160 + overlap * 8 + cramped * 2 + index,
    };
  });

  const best = scored.sort((a, b) => a.score - b.score)[0];
  return {
    x: best.rect.x,
    y: best.rect.y,
    width: best.rect.width,
    height: best.rect.height,
    placement: best.placement,
  };
}
