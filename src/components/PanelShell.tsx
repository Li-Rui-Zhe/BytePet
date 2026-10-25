import { X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { PropsWithChildren } from "react";
import { useAppStore } from "../stores/useAppStore";
import { ActionBar } from "./ActionBar";
import { getPanelLayout, useViewportSize, type Size } from "./boxLayout";

interface PanelShellProps extends PropsWithChildren {
  title: string;
  subtitle?: string;
  onClose: () => void;
  hideNavigation?: boolean;
  preferredSize?: Size;
  className?: string;
}

type ResizeEdge = "n" | "e" | "s" | "w" | "ne" | "se" | "sw" | "nw";

interface PanelRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const RESIZE_EDGES: ResizeEdge[] = ["n", "e", "s", "w", "ne", "se", "sw", "nw"];
const MIN_PANEL_WIDTH = 286;
const MIN_PANEL_HEIGHT = 286;
const VIEWPORT_MARGIN = 14;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max));
}

export function PanelShell({ title, subtitle, children, onClose, hideNavigation = false, preferredSize, className = "" }: PanelShellProps) {
  const viewport = useViewportSize();
  const boxPosition = useAppStore((s) => s.boxPosition);
  const activePanel = useAppStore((s) => s.activePanel);
  const openPanel = useAppStore((s) => s.openPanel);
  const [customSize, setCustomSize] = useState<Size | null>(null);
  const layout = getPanelLayout(boxPosition, viewport, customSize ?? preferredSize);
  const displayRect = layout;
  const resizeStartRef = useRef<{ x: number; y: number; rect: PanelRect; edge: ResizeEdge } | null>(null);

  const handleResizeMove = useCallback(
    (event: PointerEvent) => {
      const start = resizeStartRef.current;
      if (!start) return;

      const dx = event.clientX - start.x;
      const dy = event.clientY - start.y;
      const maxX = viewport.width - VIEWPORT_MARGIN;
      const maxY = viewport.height - VIEWPORT_MARGIN;
      let { x, y, width, height } = start.rect;

      if (start.edge.includes("e")) {
        width = dx + start.rect.width;
      }

      if (start.edge.includes("s")) {
        height = dy + start.rect.height;
      }

      if (start.edge.includes("w")) {
        x = start.rect.x + dx;
        width = start.rect.width - dx;
      }

      if (start.edge.includes("n")) {
        y = start.rect.y + dy;
        height = start.rect.height - dy;
      }

      if (width < MIN_PANEL_WIDTH) {
        if (start.edge.includes("w")) {
          x = start.rect.x + start.rect.width - MIN_PANEL_WIDTH;
        }
        width = MIN_PANEL_WIDTH;
      }

      if (height < MIN_PANEL_HEIGHT) {
        if (start.edge.includes("n")) {
          y = start.rect.y + start.rect.height - MIN_PANEL_HEIGHT;
        }
        height = MIN_PANEL_HEIGHT;
      }

      if (x < VIEWPORT_MARGIN) {
        width -= VIEWPORT_MARGIN - x;
        x = VIEWPORT_MARGIN;
      }

      if (y < VIEWPORT_MARGIN) {
        height -= VIEWPORT_MARGIN - y;
        y = VIEWPORT_MARGIN;
      }

      width = clamp(width, MIN_PANEL_WIDTH, maxX - x);
      height = clamp(height, MIN_PANEL_HEIGHT, maxY - y);

      setCustomSize({ width, height });
    },
    [viewport.height, viewport.width]
  );

  const stopResize = useCallback(() => {
    resizeStartRef.current = null;
    window.removeEventListener("pointermove", handleResizeMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }, [handleResizeMove]);

  const startResize = useCallback(
    (event: React.PointerEvent<HTMLButtonElement>, edge: ResizeEdge) => {
      event.preventDefault();
      event.stopPropagation();
      resizeStartRef.current = {
        x: event.clientX,
        y: event.clientY,
        rect: displayRect,
        edge,
      };
      window.addEventListener("pointermove", handleResizeMove);
      window.addEventListener("pointerup", stopResize);
      window.addEventListener("pointercancel", stopResize);
    },
    [displayRect, handleResizeMove, stopResize]
  );

  useEffect(() => stopResize, [stopResize]);

  return (
    <section
      className={`glass-panel panel-shell no-drag pointer-events-auto fixed z-30 flex flex-col text-slate-700 shadow-panel ${className}`}
      data-interactive="true"
      aria-label={title}
      data-placement={layout.placement}
      onKeyDown={(event) => {
        if (event.key !== "Escape" || event.defaultPrevented || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) return;
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }}
      style={{
        left: displayRect.x,
        top: displayRect.y,
        width: displayRect.width,
        height: displayRect.height,
      }}
    >
      <header className="panel-header">
        <div className="panel-heading">
          <h2>{title}</h2>
          {subtitle ? <p title={subtitle}>{subtitle}</p> : null}
        </div>
        <button
          className="panel-close"
          type="button"
          onClick={onClose}
          aria-label="关闭面板"
          title="关闭（Esc）"
        >
          <X size={15} />
        </button>
      </header>

      {!hideNavigation ? <div className="panel-dock">
        <ActionBar activePanel={activePanel} compact onAction={openPanel} />
      </div> : null}

      <div className="panel-content min-h-0 flex-1">{children}</div>
      {RESIZE_EDGES.map((edge) => (
        <button
          key={edge}
          className={`panel-resize-zone panel-resize-${edge}`}
          type="button"
          aria-hidden="true"
          tabIndex={-1}
          onPointerDown={(event) => startResize(event, edge)}
        />
      ))}
    </section>
  );
}
