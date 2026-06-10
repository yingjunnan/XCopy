import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

const DEFAULT_SETTINGS: AppSettings = {
  autoStart: false,
  shortcut: "Ctrl+Shift+V",
};

const isTauriRuntime = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const modifierKeys = new Set(["Control", "Shift", "Alt", "Meta"]);

const keyLabels: Record<string, string> = {
  Backquote: "`",
  Backslash: "\\",
  BracketLeft: "[",
  BracketRight: "]",
  Comma: ",",
  Equal: "=",
  Minus: "-",
  Period: ".",
  Quote: "'",
  Semicolon: ";",
  Slash: "/",
  Space: "Space",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  Escape: "Esc",
};

function keyLabelFromEvent(event: React.KeyboardEvent<HTMLButtonElement>) {
  if (modifierKeys.has(event.key)) return null;

  if (event.code.startsWith("Key")) return event.code.slice(3);
  if (event.code.startsWith("Digit")) return event.code.slice(5);
  if (event.code.startsWith("Numpad")) return event.code.replace("Numpad", "Num");
  if (/^F\d{1,2}$/.test(event.code)) return event.code;

  return keyLabels[event.code] ?? event.key;
}

function shortcutFromEvent(event: React.KeyboardEvent<HTMLButtonElement>) {
  const key = keyLabelFromEvent(event);
  if (!key) return null;

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");

  if (parts.length === 0) return null;
  parts.push(key);
  return parts.join("+");
}

const SettingsPanel: React.FC = () => {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [savedSettings, setSavedSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [recording, setRecording] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;

    async function loadSettings() {
      if (!isTauriRuntime()) {
        setLoading(false);
        return;
      }

      try {
        const result = await invoke<AppSettings>("get_app_settings");
        if (cancelled) return;
        setSettings(result);
        setSavedSettings(result);
      } catch (err) {
        if (!cancelled) setError(`读取设置失败：${String(err)}`);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    loadSettings();
    return () => {
      cancelled = true;
    };
  }, []);

  const changed = useMemo(
    () =>
      settings.autoStart !== savedSettings.autoStart ||
      settings.shortcut !== savedSettings.shortcut,
    [settings, savedSettings]
  );

  const toggleAutoStart = useCallback(() => {
    setMessage("");
    setError("");
    setSettings((current) => ({
      ...current,
      autoStart: !current.autoStart,
    }));
  }, []);

  const saveSettings = useCallback(async () => {
    setSaving(true);
    setMessage("");
    setError("");

    if (!isTauriRuntime()) {
      window.setTimeout(() => {
        setSavedSettings(settings);
        setMessage("已保存");
        setSaving(false);
      }, 200);
      return;
    }

    try {
      const saved = await invoke<AppSettings>("save_app_settings", { settings });
      setSettings(saved);
      setSavedSettings(saved);
      setMessage("已保存");
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [settings]);

  const handleShortcutKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLButtonElement>) => {
      if (!recording) return;

      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecording(false);
        return;
      }

      const shortcut = shortcutFromEvent(event);
      if (!shortcut) {
        setError("快捷键至少包含 Ctrl、Alt、Shift 或 Win 中的一个修饰键");
        return;
      }

      setSettings((current) => ({ ...current, shortcut }));
      setRecording(false);
      setMessage("");
      setError("");
    },
    [recording]
  );

  return (
    <div className="no-drag flex h-full flex-col overflow-hidden">
      <div className="flex-1 overflow-y-auto px-4 py-4">
        <div className="space-y-3">
          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">开机自启</h2>
                <p className="mt-1 text-[11px] leading-4 text-slate-500">
                  {settings.autoStart ? "已开启" : "已关闭"}
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.autoStart}
                onClick={toggleAutoStart}
                disabled={loading || saving}
                className={`
                  relative h-6 w-11 rounded-full transition-colors duration-200
                  ${settings.autoStart ? "bg-[#0067c0]" : "bg-slate-300"}
                  disabled:cursor-not-allowed disabled:opacity-60
                `}
              >
                <span
                  className={`
                    absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm
                    transition-transform duration-200
                    ${settings.autoStart ? "translate-x-5" : "translate-x-0"}
                  `}
                />
              </button>
            </div>
          </section>

          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="mb-3 flex items-center justify-between gap-3">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">打开快捷键</h2>
                <p className="mt-1 truncate text-[11px] leading-4 text-slate-500">
                  {settings.shortcut}
                </p>
              </div>
              <button
                type="button"
                onClick={() => {
                  setRecording(true);
                  setMessage("");
                  setError("");
                }}
                disabled={loading || saving}
                className="
                  flex h-8 items-center justify-center rounded-lg px-3 text-[12px] font-semibold
                  text-[#0067c0] transition-all duration-150
                  hover:bg-[#0067c0]/10 active:scale-95
                  disabled:cursor-not-allowed disabled:opacity-60
                "
              >
                修改
              </button>
            </div>

            <button
              type="button"
              onKeyDown={handleShortcutKeyDown}
              onBlur={() => setRecording(false)}
              disabled={loading || saving}
              className={`
                flex h-12 w-full items-center justify-center rounded-xl border
                px-3 font-mono text-[14px] font-semibold transition-all duration-150
                ${
                  recording
                    ? "border-[#0067c0] bg-[#0067c0]/10 text-[#005aab] shadow-[0_0_0_3px_rgba(0,103,192,0.12)]"
                    : "border-slate-900/[0.10] bg-slate-50 text-slate-800 hover:border-[#0067c0]/35"
                }
                disabled:cursor-not-allowed disabled:opacity-60
              `}
              onClick={(event) => {
                setRecording(true);
                event.currentTarget.focus();
              }}
            >
              {recording ? "等待按键" : settings.shortcut}
            </button>
          </section>
        </div>
      </div>

      <div className="border-t border-slate-200 bg-white px-4 py-3">
        {error && (
          <div className="mb-2 rounded-lg bg-[#c42b1c]/10 px-3 py-2 text-[12px] font-medium leading-5 text-[#9f2117]">
            {error}
          </div>
        )}
        {message && !error && (
          <div className="mb-2 rounded-lg bg-[#107c10]/10 px-3 py-2 text-[12px] font-medium text-[#0b6a0b]">
            {message}
          </div>
        )}
        <button
          type="button"
          onClick={saveSettings}
          disabled={loading || saving || !changed}
          className="
            flex h-10 w-full items-center justify-center rounded-xl bg-[#0067c0]
            text-[13px] font-semibold text-white shadow-[0_6px_14px_rgba(0,103,192,0.20)]
            transition-all duration-150 hover:bg-[#005aab] active:scale-[0.99]
            disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500 disabled:shadow-none
          "
        >
          {saving ? "保存中..." : changed ? "保存设置" : "无需保存"}
        </button>
      </div>
    </div>
  );
};

export default SettingsPanel;
