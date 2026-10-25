import { useEffect, useRef, useState } from "react";
import { resolveFrameUrl, resolveFrameUrlForKey, FALLBACK_FRAME_URL } from "../systems/animation/frameResolver";
import { recordDragSample, releaseVelocity, type DragSample } from "../systems/physics/dragVelocity";
import type { RuntimeAnimationState } from "../systems/stateMachine/types";

interface PetSpriteProps {
  currentState: RuntimeAnimationState;
  skinId: string;
  frameAssetsPath?: string | null;
  displayScale?: number;
  size?: number;
  position: { x: number; y: number };
  onDragStart: (offsetX: number, offsetY: number) => void;
  onDrag: (x: number, y: number) => void;
  onDragEnd: (velocityX: number, velocityY: number) => void;
}

export function PetSprite({
  currentState,
  skinId,
  frameAssetsPath,
  displayScale = 1,
  size,
  position,
  onDragStart,
  onDrag,
  onDragEnd,
}: PetSpriteProps) {
  const [frameIndex, setFrameIndex] = useState(1);
  const [imgSrc, setImgSrc] = useState<string>(FALLBACK_FRAME_URL);
  const definition = currentState.definition;
  const draggingRef = useRef(false);
  const dragOffsetRef = useRef({ x: 0, y: 0 });
  const samplesRef = useRef<DragSample[]>([]);
  const pointerRef = useRef<number | null>(null);
  const onDragEndRef = useRef(onDragEnd);
  onDragEndRef.current = onDragEnd;
  const containerRef = useRef<HTMLDivElement>(null);
  const failedFrameUrlsRef = useRef(new Set<string>());
  const safeDisplayScale = Math.min(3, Math.max(0.75, Number.isFinite(displayScale) ? displayScale : 1));
  const visualSize = size && Number.isFinite(size) ? Math.max(1, size) : Math.round(PET_DISPLAY_SIZE * safeDisplayScale);

  useEffect(() => () => {
    if (draggingRef.current) { draggingRef.current = false; onDragEndRef.current(0, 0); }
  }, []);

  const resolveCurrentCharacterCalmUrl = (index: number) =>
    resolveFrameUrlForKey({
      stateKey: "calm",
      skinId,
      frameAssetsPath,
      frameIndex: Math.min(Math.max(1, index), 6),
    });

  const getDisplayFrameUrl = (url: string, index: number) => {
    if (!failedFrameUrlsRef.current.has(url)) {
      return url;
    }
    const calmUrl = resolveCurrentCharacterCalmUrl(index);
    return failedFrameUrlsRef.current.has(calmUrl) ? FALLBACK_FRAME_URL : calmUrl;
  };

  useEffect(() => {
    failedFrameUrlsRef.current.clear();
  }, [skinId, frameAssetsPath]);

  // Frame animation
  useEffect(() => {
    setFrameIndex(1);
    const firstFrameUrl = resolveFrameUrl({ state: definition, skinId, frameAssetsPath, frameIndex: 1 });
    setImgSrc(getDisplayFrameUrl(firstFrameUrl, 1));

    const scaleFrameMultiplier = safeDisplayScale > 1 ? 1 + (safeDisplayScale - 1) * 0.35 : 1;
    const baseIntervalMs = Math.max(80, Math.round(1000 / definition.fps));
    const durationSafeIntervalMs =
      !definition.loop && definition.durationMs > 0 && definition.frames > 1
        ? Math.max(80, Math.floor((definition.durationMs - 80) / (definition.frames - 1)))
        : Number.POSITIVE_INFINITY;
    const intervalMs = Math.min(
      durationSafeIntervalMs,
      Math.max(80, Math.round(baseIntervalMs * scaleFrameMultiplier))
    );
    let timer: number | null = null;
    if (definition.frames <= 1) {
      return undefined;
    }

    timer = window.setInterval(() => {
      setFrameIndex((frame) => {
        const nextFrame = frame + 1;
        if (nextFrame <= definition.frames) {
          return nextFrame;
        }
        if (!definition.loop && timer !== null) {
          window.clearInterval(timer);
          timer = null;
        }
        return definition.loop ? 1 : definition.frames;
      });
    }, intervalMs);

    return () => {
      if (timer !== null) {
        window.clearInterval(timer);
      }
    };
  }, [definition.key, definition.fps, definition.frames, definition.loop, definition.durationMs, currentState.startedAt, skinId, frameAssetsPath, safeDisplayScale]);

  // Update image source when frame changes
  useEffect(() => {
    const url = resolveFrameUrl({ state: definition, skinId, frameAssetsPath, frameIndex });
    setImgSrc(getDisplayFrameUrl(url, frameIndex));
  }, [definition, skinId, frameAssetsPath, frameIndex]);

  function handlePointerDown(event: React.PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0 && event.button !== 2 || draggingRef.current) return;
    event.preventDefault();
    dragOffsetRef.current = { x: event.clientX - position.x, y: event.clientY - position.y };
    samplesRef.current = [{ x: event.clientX, y: event.clientY, time: performance.now() }];
    pointerRef.current = event.pointerId;
    draggingRef.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
    onDragStart(dragOffsetRef.current.x, dragOffsetRef.current.y);
  }

  function handlePointerMove(event: React.PointerEvent<HTMLDivElement>): void {
    if (!draggingRef.current || pointerRef.current !== event.pointerId) return;
    samplesRef.current = recordDragSample(samplesRef.current, { x: event.clientX, y: event.clientY, time: performance.now() });
    onDrag(event.clientX - dragOffsetRef.current.x, event.clientY - dragOffsetRef.current.y);
  }

  function finishPointer(event: React.PointerEvent<HTMLDivElement>, cancelled = false): void {
    if (!draggingRef.current || pointerRef.current !== event.pointerId) return;
    draggingRef.current = false;
    pointerRef.current = null;
    const velocity = cancelled ? { x: 0, y: 0 } : releaseVelocity(samplesRef.current, { x: event.clientX, y: event.clientY, time: performance.now() });
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    onDragEndRef.current(velocity.x, velocity.y);
  }

  // 不做 CSS 翻转，方向由 run_left/run_right 动画文件夹决定
  const flipStyle = "scaleX(1)";

  return (
    <div
      ref={containerRef}
      className="pet-sprite-area pointer-events-auto absolute z-10 flex flex-col items-center overflow-visible"
      style={{
        left: `${position.x - visualSize / 2}px`,
        top: `${position.y}px`,
        width: `${visualSize}px`,
        height: `${visualSize}px`,
      }}
      data-interactive="true"
    >
      <div
        className="absolute cursor-grab bg-white/[0.001] active:cursor-grabbing"
        style={{
          left: 0,
          top: 0,
          touchAction: "none",
          width: `${visualSize}px`,
          height: `${visualSize}px`,
          transform: flipStyle,
        }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={(event) => finishPointer(event)}
        onPointerCancel={(event) => finishPointer(event, true)}
        onLostPointerCapture={(event) => finishPointer(event, true)}
        onContextMenu={(event) => event.preventDefault()}
        data-interactive="true"
      >
        <img
          className="h-full w-full object-contain"
          src={imgSrc}
          alt="BytePet"
          draggable={false}
          onError={() => {
            failedFrameUrlsRef.current.add(imgSrc);
            const calmUrl = resolveCurrentCharacterCalmUrl(frameIndex);
            setImgSrc(imgSrc === calmUrl || failedFrameUrlsRef.current.has(calmUrl) ? FALLBACK_FRAME_URL : calmUrl);
          }}
        />
      </div>
    </div>
  );
}

const PET_DISPLAY_SIZE = 152;
