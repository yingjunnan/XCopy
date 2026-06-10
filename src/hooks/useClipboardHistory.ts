import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ClipboardEntry, ClipboardFilter, CategoryType } from "../types";

export function useClipboardHistory() {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);
  const [category, setCategory] = useState<CategoryType>("all");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

  const fetchHistory = useCallback(async (filter: ClipboardFilter) => {
    try {
      setLoading(true);
      const result = await invoke<ClipboardEntry[]>("get_history", { filter });
      setEntries(result);
    } catch (err) {
      console.error("Failed to fetch history:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const refresh = useCallback(() => {
    fetchHistory({
      query: query || undefined,
      contentType: category === "all" ? undefined : category,
      limit: 200,
      offset: 0,
    });
  }, [query, category, fetchHistory]);

  // Initial load + re-fetch on filter change
  useEffect(() => {
    refresh();
  }, [refresh]);

  // Listen for real-time clipboard updates AND window focus — both re-fetch from DB
  useEffect(() => {
    const refetch = () => {
      invoke<ClipboardEntry[]>("get_history", {
        filter: {
          query: query || undefined,
          contentType: category === "all" ? undefined : category,
          limit: 200,
          offset: 0,
        },
      })
        .then((result) => setEntries(result))
        .catch((err) => console.error("Refetch failed:", err));
    };

    const unlisten1 = listen<any>("clipboard-update", () => {
      refetch();
    });

    // Re-fetch when window regains focus (reopened via shortcut)
    const win = getCurrentWindow();
    const unlisten2 = win.onFocusChanged(({ payload: focused }) => {
      if (focused) refetch();
    });

    return () => {
      unlisten1.then((fn) => fn());
      unlisten2.then((fn) => fn());
    };
  }, [query, category]);

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
