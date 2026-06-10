import React, { useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useClipboardHistory } from "../hooks/useClipboardHistory";
import SearchBar from "./SearchBar";
import CategoryTabs from "./CategoryTabs";
import ClipboardItem from "./ClipboardItem";
import EmptyState from "./EmptyState";
import type { CategoryType } from "../types";

const ClipboardPanel: React.FC = () => {
  const {
    entries,
    allEntries,
    category,
    setCategory,
    query,
    setQuery,
    loading,
    deleteEntry,
    clearAll,
  } = useClipboardHistory();
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);
  const [clearing, setClearing] = useState(false);

  const counts = useMemo(() => {
    const c: Record<CategoryType, number> = { all: 0, text: 0, link: 0, image: 0 };
    allEntries.forEach((e) => {
      c.all++;
      c[e.contentType]++;
    });
    return c;
  }, [allEntries]);

  const activeCount = category === "all" ? counts.all : counts[category];

  const handleConfirmClear = async () => {
    setClearing(true);
    try {
      await clearAll();
      setConfirmClearOpen(false);
    } finally {
      setClearing(false);
    }
  };

  return (
    <div className="relative flex h-full flex-col">
      {/* Search */}
      <div className="pb-1 pt-3">
        <SearchBar value={query} onChange={setQuery} />
      </div>

      {/* Category tabs + clear */}
      <div className="flex items-center justify-between pr-3">
        <CategoryTabs current={category} onChange={setCategory} counts={counts} />
        {allEntries.length > 0 && (
          <button
            onClick={() => setConfirmClearOpen(true)}
            className="
              no-drag mr-1 flex-shrink-0 rounded-lg px-2.5 py-1.5 text-[11px] font-semibold
              text-slate-500 transition-all duration-200 hover:bg-[#c42b1c]/10 hover:text-[#c42b1c]
            "
          >
            清空
          </button>
        )}
      </div>

      {/* Content */}
      <div className="mt-1 flex-1 overflow-hidden">
        {/* Status bar */}
        <div className="flex items-center justify-between px-4 pb-1.5">
          <span className="text-[11px] font-medium text-slate-500">
            {loading ? "正在同步..." : `${activeCount} 条记录`}
          </span>
          {query && (
            <span className="max-w-[190px] truncate rounded-md bg-slate-100 px-2 py-0.5 text-[11px] font-medium text-slate-600">
              搜索 "{query}"
            </span>
          )}
        </div>

        {/* List */}
        <div className="h-full overflow-y-auto px-2 pb-4 scroll-smooth">
          {loading ? (
            <div className="space-y-2 px-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <div
                  key={i}
                  className="h-16 animate-pulse rounded-xl border border-slate-200 bg-white"
                  style={{ animationDelay: `${i * 50}ms` }}
                />
              ))}
            </div>
          ) : entries.length === 0 ? (
            <EmptyState />
          ) : (
            <AnimatePresence mode="popLayout">
              {entries.map((entry) => (
                <motion.div
                  key={entry.id}
                  layout
                  initial={{ opacity: 0, y: 8, scale: 0.98 }}
                  animate={{ opacity: 1, y: 0, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95, transition: { duration: 0.15 } }}
                  transition={{
                    type: "spring",
                    stiffness: 400,
                    damping: 30,
                    mass: 0.8,
                  }}
                >
                  <ClipboardItem entry={entry} onDelete={deleteEntry} />
                </motion.div>
              ))}
            </AnimatePresence>
          )}

          {/* Bottom padding for scroll */}
          <div className="h-2" />
        </div>
      </div>

      <AnimatePresence>
        {confirmClearOpen && (
          <motion.div
            className="no-drag absolute inset-0 z-30 flex items-center justify-center bg-slate-950/25 px-5"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onMouseDown={() => setConfirmClearOpen(false)}
          >
            <motion.div
              role="dialog"
              aria-modal="true"
              aria-labelledby="clear-history-title"
              className="w-full max-w-[320px] rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_18px_42px_rgba(15,23,42,0.22)]"
              initial={{ opacity: 0, scale: 0.96, y: 8 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.96, y: 8 }}
              transition={{ duration: 0.16 }}
              onMouseDown={(event) => event.stopPropagation()}
            >
              <h2 id="clear-history-title" className="text-[14px] font-semibold text-slate-900">
                清空剪贴板历史？
              </h2>
              <p className="mt-2 text-[12px] leading-5 text-slate-500">
                这会删除当前保存的所有文本、链接和图片记录，操作无法撤销。
              </p>
              <div className="mt-4 flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setConfirmClearOpen(false)}
                  disabled={clearing}
                  className="
                    flex h-8 items-center justify-center rounded-lg px-3 text-[12px] font-semibold
                    text-slate-600 transition-all duration-150 hover:bg-slate-900/[0.06]
                    disabled:cursor-not-allowed disabled:opacity-60
                  "
                >
                  取消
                </button>
                <button
                  type="button"
                  onClick={handleConfirmClear}
                  disabled={clearing}
                  className="
                    flex h-8 items-center justify-center rounded-lg bg-[#c42b1c] px-3 text-[12px] font-semibold
                    text-white shadow-[0_6px_14px_rgba(196,43,28,0.22)]
                    transition-all duration-150 hover:bg-[#a8251a] active:scale-95
                    disabled:cursor-not-allowed disabled:opacity-70
                  "
                >
                  {clearing ? "清空中..." : "确认清空"}
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default ClipboardPanel;
