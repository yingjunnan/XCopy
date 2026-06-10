import React, { useMemo } from "react";
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
    category,
    setCategory,
    query,
    setQuery,
    loading,
    deleteEntry,
    clearAll,
  } = useClipboardHistory();

  const counts = useMemo(() => {
    const c: Record<CategoryType, number> = { all: 0, text: 0, link: 0, image: 0 };
    entries.forEach((e) => {
      c.all++;
      c[e.contentType]++;
    });
    return c;
  }, [entries]);

  const activeCount = category === "all" ? counts.all : counts[category];

  return (
    <div className="flex h-full flex-col">
      {/* Search */}
      <div className="pb-1 pt-3">
        <SearchBar value={query} onChange={setQuery} />
      </div>

      {/* Category tabs + clear */}
      <div className="flex items-center justify-between pr-3">
        <CategoryTabs current={category} onChange={setCategory} counts={counts} />
        {entries.length > 0 && (
          <button
            onClick={clearAll}
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
            <span className="max-w-[190px] truncate rounded-md bg-white/[0.55] px-2 py-0.5 text-[11px] font-medium text-slate-500">
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
                  className="h-16 animate-pulse rounded-xl border border-slate-900/[0.06] bg-white/[0.58]"
                  style={{ animationDelay: `${i * 50}ms` }}
                />
              ))}
            </div>
          ) : entries.length === 0 ? (
            <EmptyState />
          ) : (
            <AnimatePresence mode="popLayout">
              {entries.map((entry, idx) => (
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
    </div>
  );
};

export default ClipboardPanel;
