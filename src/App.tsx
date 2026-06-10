import React from "react";
import { invoke } from "@tauri-apps/api/core";
import ClipboardPanel from "./components/ClipboardPanel";

const isTauriRuntime = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const App: React.FC = () => {
  const handleClose = async () => {
    if (isTauriRuntime()) {
      try {
        await invoke("hide_main_window");
      } catch (err) {
        console.error("Failed to hide window:", err);
      }
      return;
    }

    window.close();
  };

  return (
    <div
      className="
        w-full h-full overflow-hidden rounded-[18px]
        border border-slate-200
        bg-white/[0.72] text-slate-900
        shadow-[0_24px_64px_rgba(31,41,55,0.18),0_2px_8px_rgba(31,41,55,0.08),0_0_0_1px_rgba(255,255,255,0.72)_inset]
        backdrop-blur-2xl
      "
    >
      {/* Title bar — drag region */}
      <div
        className="
          flex h-10 items-center justify-between
          border-b border-slate-200
          bg-white px-3
        "
      >
        <div className="drag-region flex items-center gap-2">
          <span className="grid h-4 w-4 grid-cols-2 gap-0.5 rounded-[4px] bg-[#0067c0]/10 p-0.5 text-[#0067c0]">
            <span className="rounded-[1px] bg-current" />
            <span className="rounded-[1px] bg-current opacity-75" />
            <span className="rounded-[1px] bg-current opacity-75" />
            <span className="rounded-[1px] bg-current" />
          </span>
          <span className="select-none text-[12px] font-semibold text-slate-700">
            XCopy
          </span>
        </div>
        <span
          className="drag-region select-none text-[11px] font-semibold tracking-[0.10em] text-slate-600"
        >
          剪贴板历史
        </span>
        <div className="flex h-7 w-[60px] items-center justify-end">
          <button
            type="button"
            onClick={handleClose}
            onMouseDown={(event) => event.stopPropagation()}
            onPointerDown={(event) => event.stopPropagation()}
            title="关闭"
            aria-label="关闭窗口"
            className="
              no-drag flex h-7 w-7 items-center justify-center rounded-lg
              text-slate-500 transition-all duration-150
              hover:bg-[#c42b1c]/10 hover:text-[#c42b1c]
              active:scale-95
            "
          >
            <svg className="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Main content */}
      <div className="h-[calc(100%-40px)] bg-[radial-gradient(circle_at_16%_0%,rgba(0,120,212,0.10),transparent_34%),linear-gradient(135deg,rgba(255,255,255,0.78),rgba(244,248,252,0.74))]">
        <ClipboardPanel />
      </div>
    </div>
  );
};

export default App;
