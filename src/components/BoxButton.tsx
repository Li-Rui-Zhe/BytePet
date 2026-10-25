import { useCallback, useEffect, useRef, useState } from "react";
import { useAppStore } from "../stores/useAppStore";
import { resolveFrameUrlForKey } from "../systems/animation/frameResolver";
import { BOX_BUTTON_SIZE, clampBoxPosition, getDefaultBoxPosition, useViewportSize } from "./boxLayout";

const DRAG_THRESHOLD = 5;

export function BoxButton() {
  const viewport = useViewportSize();
  const boxPosition = useAppStore((s) => s.boxPosition);
  const boxOpen = useAppStore((s) => s.boxOpen);
  const character = useAppStore((s) => s.character);
  const toggleBox = useAppStore((s) => s.toggleBox);
  const updateBoxPosition = useAppStore((s) => s.updateBoxPosition);
  const boxSkinId = character?.skinId || "rick_default";
  const boxImageUrl = resolveFrameUrlForKey({
    stateKey: "box",
    skinId: boxSkinId,
    frameAssetsPath: character?.frameAssetsPath ?? null,
    frameIndex: 1,
  });

  const [dragging, setDragging] = useState(false);
  const dragStartRef = useRef<{ x: number; y: number; posX: number; posY: number } | null>(null);
  const movedRef = useRef(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const getPosition = useCallback(() => {
    if (boxPosition) return boxPosition;
    return getDefaultBoxPosition(viewport);
  }, [boxPosition, viewport]);

  const pos = getPosition();

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      dragStartRef.current = { x: event.clientX, y: event.clientY, posX: pos.x, posY: pos.y };
      movedRef.current = false;
      setDragging(true);
    },
    [pos.x, pos.y]
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      if (!dragStartRef.current) return;

      const dx = event.clientX - dragStartRef.current.x;
      const dy = event.clientY - dragStartRef.current.y;

      if (!movedRef.current && Math.abs(dx) + Math.abs(dy) > DRAG_THRESHOLD) {
        movedRef.current = true;
      }

      if (movedRef.current) {
        const next = clampBoxPosition(dragStartRef.current.posX + dx, dragStartRef.current.posY + dy, viewport);
        updateBoxPosition(next.x, next.y);
      }
    },
    [updateBoxPosition, viewport]
  );

  const handlePointerUp = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>) => {
      event.preventDefault();

      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }

      dragStartRef.current = null;
      setDragging(false);

      if (!movedRef.current) {
        toggleBox();
      }
    },
    [toggleBox]
  );

  useEffect(() => {
    if (!boxPosition) {
      const next = getDefaultBoxPosition(viewport);
      updateBoxPosition(next.x, next.y);
      return;
    }

    const next = clampBoxPosition(boxPosition.x, boxPosition.y, viewport);
    if (next.x !== boxPosition.x || next.y !== boxPosition.y) {
      updateBoxPosition(next.x, next.y);
    }
  }, [boxPosition, updateBoxPosition, viewport]);

  return (
    <button
      ref={buttonRef}
      className="box-launcher no-drag pointer-events-auto fixed z-50 select-none"
      data-interactive="true"
      data-open={boxOpen ? "true" : "false"}
      style={{
        left: pos.x,
        top: pos.y,
        width: BOX_BUTTON_SIZE,
        height: BOX_BUTTON_SIZE,
        cursor: dragging ? "grabbing" : "grab",
        touchAction: "none",
      }}
      type="button"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
      onKeyDown={event => {
        if (event.repeat) return;
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          toggleBox();
        }
      }}
      aria-label={boxOpen ? "收起工具箱" : "展开工具箱"}
      title={boxOpen ? "收起工具箱" : "展开工具箱"}
    >
      <img
        src={boxImageUrl}
        alt=""
        draggable={false}
        className="pointer-events-none h-full w-full object-contain"
        style={{ imageRendering: "auto" }}
        onError={(event) => {
          if (event.currentTarget.src.endsWith("/assets/skins/rick_default/box/box_0001.png")) return;
          event.currentTarget.src = "/assets/skins/rick_default/box/box_0001.png";
        }}
      />
    </button>
  );
}
