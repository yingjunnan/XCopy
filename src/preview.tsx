import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import "./index.css";

const MIN_SCALE = 0.1;
const MAX_SCALE = 8;

/** Read the image path from the URL hash (`preview.html#<encoded-path>`). */
function readPathFromHash(): string | null {
  const raw = window.location.hash.replace(/^#/, "");
  if (!raw) return null;
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

const ImagePreview: React.FC = () => {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  // zoom & pan
  const [scale, setScale] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const dragRef = useRef<{ startX: number; startY: number; baseX: number; baseY: number } | null>(
    null,
  );

  const loadImage = useCallback(async (path: string) => {
    setError(null);
    setImageSrc(null);
    setNaturalSize(null);
    setScale(1);
    setOffset({ x: 0, y: 0 });

    try {
      const data = await invoke<string>("read_image_file", { path });
      setImageSrc(`data:image/png;base64,${data}`);
    } catch (e) {
      setError(`加载图片失败：${e}`);
    }
  }, []);

  // Initial load: read path from URL hash.
  useEffect(() => {
    const path = readPathFromHash();
    if (!path) {
      setError("没有可预览的图片。");
      return;
    }
    loadImage(path);
  }, [loadImage]);

  // Allow the parent window to swap the image while this window stays open.
  useEffect(() => {
    const unlisten = listen<{ path: string }>("preview-open-image", (event) => {
      if (event.payload?.path) loadImage(event.payload.path);
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [loadImage]);

  // Esc / close — now permitted via the default capability.
  const close = useCallback(() => {
    getCurrentWindow()
      .close()
      .catch(() => {});
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  const onWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    e.preventDefault();
    setScale((s) => {
      const delta = -e.deltaY * 0.0015;
      const next = Math.min(MAX_SCALE, Math.max(MIN_SCALE, s * (1 + delta)));
      return next;
    });
  }, []);

  const onPointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    (e.currentTarget as HTMLDivElement).setPointerCapture(e.pointerId);
    dragRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      baseX: offset.x,
      baseY: offset.y,
    };
  }, [offset]);

  const onPointerMove = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag) return;
    setOffset({
      x: drag.baseX + (e.clientX - drag.startX),
      y: drag.baseY + (e.clientY - drag.startY),
    });
  }, []);

  const endDrag = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    dragRef.current = null;
    try {
      (e.currentTarget as HTMLDivElement).releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  }, []);

  const resetView = useCallback(() => {
    setScale(1);
    setOffset({ x: 0, y: 0 });
  }, []);

  const zoomBy = useCallback((factor: number) => {
    setScale((s) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s * factor)));
  }, []);

  const percent = Math.round(scale * 100);

  return (
    <div className="relative flex h-screen w-screen select-none flex-col bg-[#0b0d10] text-slate-200">
      {/* Toolbar */}
      <div className="drag-region flex h-10 flex-shrink-0 items-center justify-between border-b border-white/10 bg-[#0b0d10] px-3">
        <div className="flex items-center gap-2 text-[12px] font-medium text-slate-300">
          <span className="truncate">
            {naturalSize ? `${naturalSize.w} × ${naturalSize.h} px` : "图片预览"}
          </span>
        </div>
        <div className="no-drag flex items-center gap-1">
          <button
            onClick={() => zoomBy(1 / 1.25)}
            className="flex h-7 w-7 items-center justify-center rounded-md text-slate-300 transition hover:bg-white/10"
            title="缩小"
          >
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14" />
            </svg>
          </button>
          <button
            onClick={resetView}
            className="min-w-[3rem] rounded-md px-2 py-1 text-center text-[11px] font-semibold tabular-nums text-slate-300 transition hover:bg-white/10"
            title="重置"
          >
            {percent}%
          </button>
          <button
            onClick={() => zoomBy(1.25)}
            className="flex h-7 w-7 items-center justify-center rounded-md text-slate-300 transition hover:bg-white/10"
            title="放大"
          >
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v14M5 12h14" />
            </svg>
          </button>
          <div className="mx-1 h-5 w-px bg-white/10" />
          <button
            onClick={close}
            className="flex h-7 w-7 items-center justify-center rounded-md text-slate-300 transition hover:bg-[#c42b1c]/80 hover:text-white"
            title="关闭 (Esc)"
          >
            <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Stage */}
      <div
        className="relative flex flex-1 cursor-grab items-center justify-center overflow-hidden active:cursor-grabbing"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onDoubleClick={resetView}
      >
        {error ? (
          <div className="px-6 text-center text-[13px] text-slate-400">{error}</div>
        ) : !imageSrc ? (
          <div className="text-[13px] text-slate-500">加载图片中...</div>
        ) : (
          <img
            src={imageSrc}
            alt="预览"
            draggable={false}
            onLoad={(e) => {
              const img = e.currentTarget;
              setNaturalSize({ w: img.naturalWidth, h: img.naturalHeight });
              // Fit to window on first load.
              const stage = img.parentElement;
              if (stage) {
                const fit = Math.min(
                  (stage.clientWidth - 32) / img.naturalWidth,
                  (stage.clientHeight - 32) / img.naturalHeight,
                  1,
                );
                if (fit > 0 && fit < 1) setScale(fit);
              }
            }}
            className="max-w-none origin-center will-change-transform"
            style={{
              transform: `translate(${offset.x}px, ${offset.y}px) scale(${scale})`,
            }}
          />
        )}
      </div>

      {/* Hint footer */}
      <div className="pointer-events-none absolute bottom-2 left-1/2 -translate-x-1/2 rounded-full bg-black/50 px-3 py-1 text-[10px] font-medium text-slate-400">
        滚轮缩放 · 拖动平移 · 双击复位 · Esc 关闭
      </div>
    </div>
  );
};

createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ImagePreview />
  </React.StrictMode>,
);
