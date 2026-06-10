import React from "react";
import type { CategoryType } from "../types";

interface CategoryTabsProps {
  current: CategoryType;
  onChange: (cat: CategoryType) => void;
  counts: Record<CategoryType, number>;
}

const categories: { key: CategoryType; label: string; icon: string }[] = [
  { key: "all", label: "全部", icon: "⊞" },
  { key: "text", label: "文本", icon: "☰" },
  { key: "link", label: "链接", icon: "⛓" },
  { key: "image", label: "图片", icon: "▣" },
];

const CategoryTabs: React.FC<CategoryTabsProps> = ({ current, onChange, counts }) => {
  return (
    <div className="flex gap-1 px-3 py-2 overflow-x-auto no-drag">
      {categories.map((cat) => (
        <button
          key={cat.key}
          onClick={() => onChange(cat.key)}
          className={`
            relative flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium
            transition-all duration-200 whitespace-nowrap
            ${
              current === cat.key
                ? "bg-white/10 text-white shadow-sm"
                : "text-white/50 hover:text-white/80 hover:bg-white/5"
            }
          `}
        >
          <span className="text-[10px] opacity-60">{cat.icon}</span>
          <span>{cat.label}</span>
          {counts[cat.key] > 0 && (
            <span
              className={`
                ml-0.5 text-[10px] px-1.5 py-0.5 rounded-full min-w-[18px] text-center
                ${current === cat.key ? "bg-white/15" : "bg-white/5"}
              `}
            >
              {counts[cat.key]}
            </span>
          )}
        </button>
      ))}
    </div>
  );
};

export default CategoryTabs;
