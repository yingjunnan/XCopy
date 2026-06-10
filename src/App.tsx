import React from "react";
import ClipboardPanel from "./components/ClipboardPanel";

const App: React.FC = () => {
  return (
    <div
      className="
        w-full h-full overflow-hidden rounded-[18px]
        border border-slate-900/[0.10]
        bg-white/[0.72] text-slate-900
        shadow-[0_24px_64px_rgba(31,41,55,0.18),0_2px_8px_rgba(31,41,55,0.08),0_0_0_1px_rgba(255,255,255,0.72)_inset]
        backdrop-blur-2xl
      "
    >
      {/* Title bar — drag region */}
      <div
        data-tauri-drag-region
        className="
          drag-region flex h-10 items-center justify-between
          border-b border-slate-900/[0.08]
          bg-white/[0.58] px-3
        "
      >
        <div className="flex items-center gap-2">
          <span className="grid h-4 w-4 grid-cols-2 gap-0.5 rounded-[4px] bg-[#0067c0]/10 p-0.5 text-[#0067c0]">
            <span className="rounded-[1px] bg-current" />
            <span className="rounded-[1px] bg-current opacity-75" />
            <span className="rounded-[1px] bg-current opacity-75" />
            <span className="rounded-[1px] bg-current" />
          </span>
          <span className="select-none text-[12px] font-semibold text-slate-600">
            XCopy
          </span>
        </div>
        <span
          data-tauri-drag-region={false}
          className="no-drag select-none text-[11px] font-medium tracking-[0.10em] text-slate-400"
        >
          剪贴板历史
        </span>
        <div className="h-6 w-[60px]" />
      </div>

      {/* Main content */}
      <div className="h-[calc(100%-40px)] bg-[radial-gradient(circle_at_16%_0%,rgba(0,120,212,0.10),transparent_34%),linear-gradient(135deg,rgba(255,255,255,0.78),rgba(244,248,252,0.74))]">
        <ClipboardPanel />
      </div>
    </div>
  );
};

export default App;
