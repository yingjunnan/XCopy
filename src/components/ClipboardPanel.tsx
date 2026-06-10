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

  return (
    <div className="flex flex-col h-full">
      {/* Search */}
      <div className="pt-2 pb-1">
        <SearchBar value={query} onChange={setQuery} />
      </div>

      {/* Category tabs + clear */}
      <div className="flex items-center justify-between pr-2">
        <CategoryTabs current={category} onChange={setCategory} counts={counts} />
        {entries.length > 0 && (
          <button
            onClick={clearAll}
            className="
              flex-shrink-0 mr-1 px-2 py-1 rounded-lg text-[10px] text-white/20
              hover:text-red-400 hover:bg-red-500/5 transition-all duration-200
              no-drag
            "
          >
            清空
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden mt-1">
        {/* Status bar */}
        <div className="px-3 pb-1 flex items-center justify-between">
          <span className="text-[10px] text-white/15">
            {loading ? "加载中..." : `${counts.all} 条记录`}
          </span>
          {query && (
            <span className="text-[10px] text-white/25">
              搜索 "{query}"
            </span>
          )}
        </div>

        {/* List */}
        <div className="overflow-y-auto h-full pb-4 px-1 scroll-smooth">
          {loading ? (
            <div className="space-y-2 px-2">
              {Array.from({ length: 5 }).map((_, i) => (
                <div
                  key={i}
                  className="h-16 rounded-xl bg-white/[0.02] animate-pulse"
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
