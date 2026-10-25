import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function useClickThrough() {
  const lastIgnoreState = useRef<boolean | null>(null);
  const isCheckingRef = useRef(false);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let disposed = false;
    const reset = () => { lastIgnoreState.current = null; };
    const unlisten = listen("cursor-state-reset", reset).catch(() => () => {});
    window.addEventListener("focus", reset);
    const checkMouse = async () => {
      if (isCheckingRef.current) return;
      isCheckingRef.current = true;

      try {
        const [x, y] = await invoke<[number, number]>("get_mouse_pos");
        if (disposed) return;
        const target = document.elementFromPoint(x, y) as HTMLElement | null;

        let shouldIgnore = true;

        if (!target) {
          shouldIgnore = true;
        } else {
          // Check if cursor is over an interactive element (pet, panels, buttons)
          const interactiveEl = target.closest("[data-interactive]");
          if (interactiveEl) {
            // For pet sprite area, do alpha detection
            const petContainer = target.closest(".pet-sprite-area");
            if (petContainer) {
              const media = petContainer.querySelector("img");
              if (media) {
                const rect = media.getBoundingClientRect();
                const mediaX = x - rect.left;
                const mediaY = y - rect.top;

                if (rect.width > 0 && rect.height > 0) {
                  if (!canvasRef.current) {
                    canvasRef.current = document.createElement("canvas");
                    canvasRef.current.width = 1;
                    canvasRef.current.height = 1;
                    ctxRef.current = canvasRef.current.getContext("2d", { willReadFrequently: true });
                  }

                  const ctx = ctxRef.current;
                  if (ctx) {
                    const naturalWidth = media.naturalWidth;
                    const naturalHeight = media.naturalHeight;

                    if (naturalWidth > 0 && naturalHeight > 0) {
                      try {
                        const scaleX = naturalWidth / rect.width;
                        const scaleY = naturalHeight / rect.height;

                        ctx.clearRect(0, 0, 1, 1);
                        ctx.drawImage(
                          media,
                          mediaX * scaleX,
                          mediaY * scaleY,
                          1,
                          1,
                          0,
                          0,
                          1,
                          1
                        );

                        const pixel = ctx.getImageData(0, 0, 1, 1).data;
                        const alpha = pixel[3];

                        if (alpha > 10) {
                          shouldIgnore = false;
                        }
                      } catch {
                        // Local asset-protocol images can be blocked from pixel reads.
                        // Fall back to box-level hit testing so imported characters remain draggable.
                        shouldIgnore = false;
                      }
                    } else {
                      shouldIgnore = false;
                    }
                  }
                }
              }
            } else {
              // Non-pet interactive element (panel, button, etc.)
              shouldIgnore = false;
            }
          }
        }

        if (lastIgnoreState.current !== shouldIgnore) {
          await invoke("set_ignore_cursor_events", { ignore: shouldIgnore });
          lastIgnoreState.current = shouldIgnore;
        }
      } catch {
        // Browser preview has no Tauri
      } finally {
        isCheckingRef.current = false;
      }
    };

    const interval = setInterval(checkMouse, 100);
    return () => { disposed = true; clearInterval(interval); window.removeEventListener("focus", reset); void unlisten.then(fn => fn()).catch(() => {}); };
  }, []);
}
