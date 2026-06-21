import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import type { ClipboardEntry, ClipboardFilter } from "./types";

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const QuickPastePanel: React.FC = () => {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(true);
  const listRef = useRef<HTMLDivElement>(null);

  const loadHistory = useCallback(async () => {
    if (!isTauriRuntime()) {
      setLoading(false);
      return;
    }
    try {
      const filter: ClipboardFilter = { limit: 50, offset: 0 };
      const all = await invoke<ClipboardEntry[]>("get_history", { filter });
      // 排除图片:quick-paste 只处理文本/链接
      const filtered = all.filter((e) => e.contentType !== "image");
      setEntries(filtered);
      setSelected(0);
    } catch (err) {
      console.error("Failed to load history:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const paste = useCallback(async (entry: ClipboardEntry) => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("paste_from_quick_paste", { content: entry.content });
    } catch (err) {
      console.error("Paste failed:", err);
    }
  }, []);

  const close = useCallback(() => {
    if (isTauriRuntime()) {
      getCurrentWindow().hide().catch(() => {});
    }
  }, []);

  // 键盘导航:↑↓ 选择,Enter 粘贴,Esc 关闭
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((s) => Math.min(s + 1, entries.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const entry = entries[selected];
        if (entry) paste(entry);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [entries, selected, paste, close]);

  // 失焦隐藏由 Rust 端 on_window_event 处理(立即隐藏,无延迟),
  // 不在前端维护 onFocusChanged 状态机——那套闸门逻辑在 alwaysOnTop
  // 窗口上会挡掉真正的失焦事件(详见 bugfix commit)。

  // 选中项滚动入视图
  useEffect(() => {
    const container = listRef.current;
    if (!container) return;
    const el = container.children[selected] as HTMLElement | undefined;
    if (el) {
      el.scrollIntoView({ block: "nearest" });
    }
  }, [selected]);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-xl bg-white shadow-2xl ring-1 ring-slate-200">
      <div ref={listRef} className="flex-1 overflow-y-auto py-1">
        {loading ? (
          <div className="px-4 py-8 text-center text-[12px] text-slate-400">
            加载中...
          </div>
        ) : entries.length === 0 ? (
          <div className="px-4 py-8 text-center text-[12px] text-slate-400">
            暂无文本记录
          </div>
        ) : (
          entries.map((entry, index) => (
            <div
              key={entry.id}
              onClick={() => paste(entry)}
              onMouseEnter={() => setSelected(index)}
              className={`
                cursor-pointer border-l-2 px-3 py-2 transition-colors
                ${
                  index === selected
                    ? "border-[#0067c0] bg-[#0067c0]/[0.08]"
                    : "border-transparent hover:bg-slate-50"
                }
              `}
            >
              <div className="flex items-center gap-2">
                <span
                  className={`h-1.5 w-1.5 flex-shrink-0 rounded-full ${
                    entry.contentType === "link" ? "bg-[#107c10]" : "bg-[#0067c0]"
                  }`}
                />
                <span className="min-w-0 flex-1 truncate text-[12px] leading-5 text-slate-800">
                  {entry.preview}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
      <div className="border-t border-slate-100 px-3 py-1.5 text-[10px] text-slate-400">
        ↑↓ 选择 · Enter 粘贴 · Esc 关闭
      </div>
    </div>
  );
};

createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QuickPastePanel />
  </React.StrictMode>
);
