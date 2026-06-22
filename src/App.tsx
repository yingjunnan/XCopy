import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ClipboardPanel from "./components/ClipboardPanel";
import SettingsPanel from "./components/SettingsPanel";
import { checkForUpdates, type UpdateCheckResult } from "./updateChecker";

const isTauriRuntime = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const App: React.FC = () => {
  const [view, setView] = useState<"history" | "settings">("history");
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResult | null>(null);
  const [checkingForUpdates, setCheckingForUpdates] = useState(false);

  const runUpdateCheck = useCallback(async () => {
    setCheckingForUpdates(true);
    try {
      const result = await checkForUpdates(__APP_VERSION__);
      setUpdateInfo(result);
      return result;
    } catch (err) {
      console.warn("Check for updates failed", err);
      return null;
    } finally {
      setCheckingForUpdates(false);
    }
  }, []);

  useEffect(() => {
    runUpdateCheck();
  }, [runUpdateCheck]);

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
      onContextMenu={(event) => event.preventDefault()}
      className="
        relative h-full w-full overflow-hidden rounded-[18px]
        bg-white text-slate-900
      "
    >
      <div className="pointer-events-none absolute inset-0 z-20 rounded-[18px] ring-1 ring-inset ring-slate-300" />

      <div
        className="
          grid h-10 grid-cols-[96px_1fr_96px] items-center
          border-b border-slate-200
          bg-white px-3
        "
      >
        <div className="drag-region flex h-full items-center gap-2">
          <img
            src="/xcopy.png"
            alt=""
            draggable={false}
            className="h-5 w-5 select-none rounded-[5px] object-cover"
          />
          <span className="select-none text-[12px] font-semibold text-slate-700">
            XCopy
          </span>
        </div>

        <div className="drag-region flex h-full items-center justify-center">
          <span className="select-none text-[11px] font-semibold tracking-[0.10em] text-slate-600">
            {view === "settings" ? "设置" : "剪贴板历史"}
          </span>
        </div>

        <div className="no-drag flex h-full items-center justify-end gap-1">
          <button
            type="button"
            onClick={() => setView((current) => (current === "history" ? "settings" : "history"))}
            onMouseDown={(event) => event.stopPropagation()}
            onPointerDown={(event) => event.stopPropagation()}
            title={view === "history" ? "设置" : "返回"}
            aria-label={view === "history" ? "打开设置" : "返回剪贴板历史"}
            className="
              no-drag flex h-7 w-7 items-center justify-center rounded-lg
              text-slate-500 transition-all duration-150
              hover:bg-slate-900/[0.06] hover:text-[#0067c0]
              active:scale-95
            "
          >
            {view === "history" ? (
              <svg className="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                />
              </svg>
            ) : (
              <svg className="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            )}
          </button>

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

      <div className="h-[calc(100%-40px)] bg-slate-50">
        {view === "history" ? (
          <ClipboardPanel />
        ) : (
          <SettingsPanel
            appVersion={__APP_VERSION__}
            checkingForUpdates={checkingForUpdates}
            updateInfo={updateInfo}
            onCheckForUpdates={runUpdateCheck}
          />
        )}
      </div>
    </div>
  );
};

export default App;
