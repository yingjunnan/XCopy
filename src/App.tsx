import React from "react";
import ClipboardPanel from "./components/ClipboardPanel";

const App: React.FC = () => {
  return (
    <div
      className="
        w-full h-full rounded-2xl overflow-hidden
        bg-[#12121f]/95 backdrop-blur-2xl
        border border-white/[0.06]
        shadow-[0_8px_32px_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.03)_inset]
      "
    >
      {/* Title bar — drag region */}
      <div
        data-tauri-drag-region
        className="
          drag-region h-9 flex items-center justify-between
          px-3 border-b border-white/[0.04]
          bg-white/[0.01]
        "
      >
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-red-500/70" />
          <div className="w-2 h-2 rounded-full bg-amber-500/70" />
          <div className="w-2 h-2 rounded-full bg-emerald-500/70" />
        </div>
        <span data-tauri-drag-region={false} className="text-[11px] text-white/20 font-medium tracking-wider no-drag select-none">
          XCOPY
        </span>
        <div className="w-12" />
      </div>

      {/* Main content */}
      <div className="h-[calc(100%-36px)]">
        <ClipboardPanel />
      </div>
    </div>
  );
};

export default App;
