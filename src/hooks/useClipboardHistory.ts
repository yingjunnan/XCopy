import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ClipboardEntry, ClipboardFilter, CategoryType } from "../types";

declare global {
  interface Window {
    __XCOPY_REFRESH_HISTORY?: () => void;
  }
}

export function useClipboardHistory() {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);
  const [category, setCategory] = useState<CategoryType>("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

  const fetchHistory = useCallback(async (filter: ClipboardFilter, showLoading = true) => {
    try {
      if (showLoading) setLoading(true);
      const result = await invoke<ClipboardEntry[]>("get_history", { filter });
      setEntries(result);
    } catch (err) {
      console.error("Failed to fetch history:", err);
    } finally {
      if (showLoading) setLoading(false);
    }
  }, []);

  const buildFilter = useCallback(
    (): ClipboardFilter => ({
      query: query || undefined,
      contentType: category === "all" ? undefined : category,
      limit: 200,
      offset: 0,
    }),
    [query, category]
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
    try {
      await invoke("delete_entry", { id });
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (err) {
      console.error("Failed to delete entry:", err);
    }
  }, []);

  const clearAll = useCallback(async () => {
    try {
      await invoke("clear_history");
      setEntries([]);
    } catch (err) {
      console.error("Failed to clear history:", err);
    }
  }, []);

  return {
    entries,
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
