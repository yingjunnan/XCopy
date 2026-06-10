import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ClipboardEntry, ClipboardFilter, CategoryType } from "../types";

declare global {
  interface Window {
    __XCOPY_REFRESH_HISTORY?: () => void;
  }
}

const isTauriRuntime = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const previewEntries: ClipboardEntry[] = [
  {
    id: "preview-text",
    contentType: "text",
    content: "Windows 亚克力风格预览内容",
    sourceApp: "Notepad",
    preview: "Windows 亚克力风格预览内容，列表项会以浅色玻璃层展示，并保留清晰的文字层级。",
    createdAt: new Date(Date.now() - 4 * 60 * 1000).toISOString(),
  },
  {
    id: "preview-link",
    contentType: "link",
    content: "https://learn.microsoft.com/windows/apps/design/signature-experiences/materials",
    sourceApp: "Microsoft Edge",
    preview: "https://learn.microsoft.com/windows/apps/design/signature-experiences/materials",
    createdAt: new Date(Date.now() - 38 * 60 * 1000).toISOString(),
  },
  {
    id: "preview-note",
    contentType: "text",
    content: "Ctrl+Shift+V 打开 XCopy",
    sourceApp: "XCopy",
    preview: "Ctrl+Shift+V 打开 XCopy，搜索、分类和清空操作都保持轻量桌面浮窗体验。",
    createdAt: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString(),
  },
];

export function useClipboardHistory() {
  const [allEntries, setAllEntries] = useState<ClipboardEntry[]>([]);
  const [category, setCategory] = useState<CategoryType>("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

  const fetchHistory = useCallback(async (filter: ClipboardFilter, showLoading = true) => {
    if (!isTauriRuntime()) {
      const normalizedQuery = filter.query?.trim().toLowerCase();
      const nextEntries = previewEntries.filter((entry) => {
        const matchesQuery =
          !normalizedQuery ||
          entry.preview.toLowerCase().includes(normalizedQuery) ||
          entry.sourceApp.toLowerCase().includes(normalizedQuery);

        return matchesQuery;
      });

      setAllEntries(nextEntries);
      setLoading(false);
      return;
    }

    try {
      if (showLoading) setLoading(true);
      const result = await invoke<ClipboardEntry[]>("get_history", {
        filter: { ...filter, contentType: undefined },
      });
      setAllEntries(result);
    } catch (err) {
      console.error("Failed to fetch history:", err);
    } finally {
      if (showLoading) setLoading(false);
    }
  }, []);

  const buildFilter = useCallback(
    (): ClipboardFilter => ({
      query: query || undefined,
      limit: 200,
      offset: 0,
    }),
    [query]
  );

  const entries = useMemo(
    () =>
      category === "all"
        ? allEntries
        : allEntries.filter((entry) => entry.contentType === category),
    [allEntries, category]
  );

  const refresh = useCallback(
    (showLoading = true) => {
      fetchHistory(buildFilter(), showLoading);
    },
    [buildFilter, fetchHistory]
  );

  // Initial load + re-fetch on filter change.
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Shortcut opens can happen before the clipboard polling thread has written
  // the new item, so refresh once immediately and once after the next poll.
  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const pendingTimers = new Set<number>();

    const refetch = () => {
      refresh(false);
    };

    const refetchAfterClipboardPoll = () => {
      refetch();
      const timer = window.setTimeout(() => {
        pendingTimers.delete(timer);
        refetch();
      }, 650);
      pendingTimers.add(timer);
    };

    const previousRefreshHandler = window.__XCOPY_REFRESH_HISTORY;
    window.__XCOPY_REFRESH_HISTORY = refetchAfterClipboardPoll;

    const win = getCurrentWindow();
    let checkingVisibility = false;
    const visibilityPoll = window.setInterval(async () => {
      if (checkingVisibility) return;

      checkingVisibility = true;
      try {
        if (await win.isVisible()) refetch();
      } catch (err) {
        console.error("Visibility refresh failed:", err);
      } finally {
        checkingVisibility = false;
      }
    }, 1000);

    const unlistenClipboardUpdate = listen("clipboard-update", () => {
      refetch();
    });

    const unlistenWindowShown = listen("window-shown", () => {
      refetchAfterClipboardPoll();
    });

    const unlistenFocusChanged = win.onFocusChanged(({ payload: focused }) => {
      if (focused) refetchAfterClipboardPoll();
    });

    return () => {
      window.__XCOPY_REFRESH_HISTORY = previousRefreshHandler;
      window.clearInterval(visibilityPoll);
      pendingTimers.forEach((timer) => window.clearTimeout(timer));
      unlistenClipboardUpdate.then((fn) => fn());
      unlistenWindowShown.then((fn) => fn());
      unlistenFocusChanged.then((fn) => fn());
    };
  }, [refresh]);

  const deleteEntry = useCallback(async (id: string) => {
    if (!isTauriRuntime()) {
      setAllEntries((prev) => prev.filter((e) => e.id !== id));
      return;
    }

    try {
      await invoke("delete_entry", { id });
      setAllEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (err) {
      console.error("Failed to delete entry:", err);
    }
  }, []);

  const clearAll = useCallback(async () => {
    if (!isTauriRuntime()) {
      setAllEntries([]);
      return;
    }

    try {
      await invoke("clear_history");
      setAllEntries([]);
    } catch (err) {
      console.error("Failed to clear history:", err);
    }
  }, []);

  return {
    entries,
    allEntries,
    category,
    setCategory,
    query,
    setQuery,
    loading,
    refresh,
    deleteEntry,
    clearAll,
  };
}
