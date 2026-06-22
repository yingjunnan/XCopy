import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppSettings, StorageUsage } from "../types";
import { getUpdateOpenUrl } from "../updateChecker";
import type { UpdateCheckResult } from "../updateChecker";

const DEFAULT_SETTINGS: AppSettings = {
  autoStart: false,
  shortcut: "Ctrl+Shift+V",
  maxHistoryEntries: 1000,
  retentionDays: 30,
  quickPasteEnabled: true,
  doubleClickIntervalMs: 300,
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

function clampNumber(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024))
  );
  const value = bytes / Math.pow(1024, exponent);
  const rounded = exponent === 0 ? value.toFixed(0) : value.toFixed(1);
  return `${rounded} ${units[exponent]}`;
}

interface SettingsPanelProps {
  appVersion: string;
  checkingForUpdates: boolean;
  updateInfo: UpdateCheckResult | null;
  onCheckForUpdates: () => Promise<UpdateCheckResult | null>;
}

const SettingsPanel: React.FC<SettingsPanelProps> = ({
  appVersion,
  checkingForUpdates,
  updateInfo,
  onCheckForUpdates,
}) => {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [savedSettings, setSavedSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [recording, setRecording] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [updateMessage, setUpdateMessage] = useState("");
  const [storage, setStorage] = useState<StorageUsage | null>(null);

  const loadStorageUsage = useCallback(async () => {
    if (!isTauriRuntime()) return;
    try {
      const usage = await invoke<StorageUsage>("get_storage_usage");
      setStorage(usage);
    } catch (err) {
      // Storage usage is informational; never surface as a hard error.
      console.warn("读取存储占用失败", String(err));
    }
  }, []);

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
    loadStorageUsage();
    return () => {
      cancelled = true;
    };
  }, [loadStorageUsage]);

  const changed = useMemo(
    () =>
      settings.autoStart !== savedSettings.autoStart ||
      settings.shortcut !== savedSettings.shortcut ||
      settings.maxHistoryEntries !== savedSettings.maxHistoryEntries ||
      settings.retentionDays !== savedSettings.retentionDays ||
      settings.quickPasteEnabled !== savedSettings.quickPasteEnabled,
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

  const toggleQuickPaste = useCallback(() => {
    setMessage("");
    setError("");
    setSettings((current) => ({
      ...current,
      quickPasteEnabled: !current.quickPasteEnabled,
    }));
  }, []);

  const openWebsite = useCallback(async () => {
    try {
      await openUrl("https://xcopy.debugmy.com");
    } catch (err) {
      // Opening the browser is best-effort; never block the user on it.
      console.warn("打开官网失败", String(err));
    }
  }, []);

  const openUpdateLink = useCallback(async () => {
    const url = getUpdateOpenUrl(updateInfo);
    if (!url) return;

    try {
      await openUrl(url);
    } catch (err) {
      setUpdateMessage("无法打开浏览器，请手动访问 GitHub Releases 页面");
      console.warn("鎵撳紑鏇存柊閾炬帴澶辫触", String(err));
    }
  }, [updateInfo]);

  const handleCheckForUpdates = useCallback(async () => {
    setUpdateMessage("");
    const result = await onCheckForUpdates();
    if (!result) return;

    if (!result.hasUpdate) {
      setUpdateMessage("已是最新版本");
    }
  }, [onCheckForUpdates]);

  const updateNumericSetting = useCallback(
    (key: "maxHistoryEntries" | "retentionDays", value: string) => {
      setMessage("");
      setError("");
      const parsed = Number(value);
      const min = 1;
      const max = key === "maxHistoryEntries" ? 100000 : 3650;
      setSettings((current) => ({
        ...current,
        [key]: clampNumber(parsed, min, max),
      }));
    },
    []
  );

  const saveSettings = useCallback(async () => {
    setSaving(true);
    setMessage("");
    setError("");

    const normalizedSettings: AppSettings = {
      ...settings,
      maxHistoryEntries: clampNumber(settings.maxHistoryEntries, 1, 100000),
      retentionDays: clampNumber(settings.retentionDays, 1, 3650),
    };

    if (!isTauriRuntime()) {
      window.setTimeout(() => {
        setSettings(normalizedSettings);
        setSavedSettings(normalizedSettings);
        setMessage("已保存");
        setSaving(false);
      }, 200);
      return;
    }

    try {
      const saved = await invoke<AppSettings>("save_app_settings", {
        settings: normalizedSettings,
      });
      setSettings(saved);
      setSavedSettings(saved);
      setMessage("已保存");
      // Retention changes may prune images/rows, refresh the storage display.
      loadStorageUsage();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }, [settings, loadStorageUsage]);

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
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">双击 Ctrl 快速粘贴</h2>
                <p className="mt-1 text-[11px] leading-4 text-slate-500">
                  {settings.quickPasteEnabled ? "已开启" : "已关闭"} · 双击 Ctrl 在光标处弹出选择条
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.quickPasteEnabled}
                onClick={toggleQuickPaste}
                disabled={loading || saving}
                className={`
                  relative h-6 w-11 rounded-full transition-colors duration-200
                  ${settings.quickPasteEnabled ? "bg-[#0067c0]" : "bg-slate-300"}
                  disabled:cursor-not-allowed disabled:opacity-60
                `}
              >
                <span
                  className={`
                    absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm
                    transition-transform duration-200
                    ${settings.quickPasteEnabled ? "translate-x-5" : "translate-x-0"}
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

          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="mb-3">
              <h2 className="text-[13px] font-semibold text-slate-900">历史保留</h2>
              <p className="mt-1 text-[11px] leading-4 text-slate-500">
                推荐保留 1000 条、30 天。超出任一条件会自动清理。
              </p>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <label className="block">
                <span className="mb-1 block text-[11px] font-semibold text-slate-500">
                  最多条数
                </span>
                <input
                  type="number"
                  min={1}
                  max={100000}
                  step={100}
                  value={settings.maxHistoryEntries}
                  onChange={(event) =>
                    updateNumericSetting("maxHistoryEntries", event.target.value)
                  }
                  disabled={loading || saving}
                  className="
                    h-10 w-full rounded-xl border border-slate-900/[0.10] bg-slate-50 px-3
                    text-[13px] font-semibold text-slate-900 outline-none
                    transition-all duration-150 focus:border-[#0067c0]/50
                    focus:shadow-[0_0_0_3px_rgba(0,103,192,0.12)]
                    disabled:cursor-not-allowed disabled:opacity-60
                  "
                />
              </label>
              <label className="block">
                <span className="mb-1 block text-[11px] font-semibold text-slate-500">
                  保留天数
                </span>
                <input
                  type="number"
                  min={1}
                  max={3650}
                  step={1}
                  value={settings.retentionDays}
                  onChange={(event) =>
                    updateNumericSetting("retentionDays", event.target.value)
                  }
                  disabled={loading || saving}
                  className="
                    h-10 w-full rounded-xl border border-slate-900/[0.10] bg-slate-50 px-3
                    text-[13px] font-semibold text-slate-900 outline-none
                    transition-all duration-150 focus:border-[#0067c0]/50
                    focus:shadow-[0_0_0_3px_rgba(0,103,192,0.12)]
                    disabled:cursor-not-allowed disabled:opacity-60
                  "
                />
              </label>
            </div>
          </section>

          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="mb-3">
              <h2 className="text-[13px] font-semibold text-slate-900">存储占用</h2>
              <p className="mt-1 text-[11px] leading-4 text-slate-500">
                数据库与图片分别占用的磁盘空间。
              </p>
            </div>
            <StorageRow
              label="数据库"
              value={storage?.databaseBytes}
            />
            <div className="my-2 h-px bg-slate-900/[0.06]" />
            <StorageRow
              label="图片"
              value={storage?.imagesBytes}
              isLast
            />
          </section>

          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">软件更新</h2>
                <p className="mt-1 text-[11px] leading-4 text-slate-500">
                  当前版本 v{appVersion}
                  {updateInfo?.hasUpdate && updateInfo.latestVersion
                    ? ` · 发现 v${updateInfo.latestVersion}`
                    : ""}
                </p>
                {updateMessage && !updateInfo?.hasUpdate && (
                  <p className="mt-2 text-[11px] font-medium text-[#107c10]">
                    {updateMessage}
                  </p>
                )}
              </div>
              <div className="flex flex-shrink-0 flex-col items-end gap-2">
                {updateInfo?.hasUpdate ? (
                  <button
                    type="button"
                    onClick={openUpdateLink}
                    disabled={!getUpdateOpenUrl(updateInfo)}
                    className="
                      flex h-8 items-center justify-center rounded-lg bg-[#0067c0] px-3
                      text-[12px] font-semibold text-white transition-all duration-150
                      hover:bg-[#005aab] active:scale-95 disabled:cursor-not-allowed
                      disabled:bg-slate-300 disabled:text-slate-500
                    "
                  >
                    查看更新
                  </button>
                ) : null}
                <button
                  type="button"
                  onClick={handleCheckForUpdates}
                  disabled={checkingForUpdates}
                  className="
                    flex h-8 items-center justify-center rounded-lg px-3 text-[12px] font-semibold
                    text-[#0067c0] transition-all duration-150
                    hover:bg-[#0067c0]/10 active:scale-95
                    disabled:cursor-not-allowed disabled:opacity-60
                  "
                >
                  {checkingForUpdates ? "检查中..." : "检查更新"}
                </button>
              </div>
            </div>
          </section>

          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">官网</h2>
                <p className="mt-1 text-[11px] leading-4 text-slate-500">
                  查看介绍、下载最新版本
                </p>
              </div>
              <a
                role="button"
                onClick={openWebsite}
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    openWebsite();
                  }
                }}
                className="
                  no-drag flex h-8 flex-shrink-0 cursor-pointer items-center gap-1 rounded-lg px-3
                  text-[12px] font-semibold text-[#0067c0] transition-all duration-150
                  hover:bg-[#0067c0]/10 active:scale-95
                "
              >
                xcopy.debugmy.com
                <svg className="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M14 5h5v5M19 5l-9 9M19 14v5a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h5"
                  />
                </svg>
              </a>
            </div>
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

const StorageRow: React.FC<{
  label: string;
  value: number | undefined;
  isLast?: boolean;
}> = ({ label, value, isLast }) => {
  const loading = value === undefined;
  return (
    <div className={`flex items-center justify-between gap-3 ${isLast ? "" : ""}`}>
      <span className="text-[12px] font-medium text-slate-600">{label}</span>
      <span
        className={`text-[12px] font-semibold tabular-nums ${
          loading ? "text-slate-400" : "text-slate-900"
        }`}
      >
        {loading ? "计算中…" : formatBytes(value)}
      </span>
    </div>
  );
};

export default SettingsPanel;
