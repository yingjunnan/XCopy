import React from "react";
import type { CategoryType } from "../types";

interface CategoryTabsProps {
  current: CategoryType;
  onChange: (cat: CategoryType) => void;
  counts: Record<CategoryType, number>;
}

const categories: { key: CategoryType; label: string; icon: string }[] = [
  { key: "all", label: "全部", icon: "M4 6.5h16M4 12h16M4 17.5h16" },
  { key: "text", label: "文本", icon: "M7 6h10M7 10h10M7 14h7M7 18h5" },
  { key: "link", label: "链接", icon: "M10.5 13.5l3-3m-1.4-3.6l.7-.7a3.2 3.2 0 114.5 4.5l-1.8 1.8m-3.6 3.6l-.7.7a3.2 3.2 0 11-4.5-4.5l1.8-1.8" },
  { key: "image", label: "图片", icon: "M5 6.5A1.5 1.5 0 016.5 5h11A1.5 1.5 0 0119 6.5v11a1.5 1.5 0 01-1.5 1.5h-11A1.5 1.5 0 015 17.5v-11zM8 15l2.2-2.2 1.8 1.8 2.8-3.1L17 15M8.5 9.5h.01" },
];

const CategoryTabs: React.FC<CategoryTabsProps> = ({ current, onChange, counts }) => {
  return (
    <div className="no-drag flex gap-1 overflow-x-auto px-3 py-2">
      {categories.map((cat) => {
        const count = counts[cat.key];
        const isActive = current === cat.key;

        return (
          <button
            key={cat.key}
            onClick={() => onChange(cat.key)}
            aria-label={`${cat.label}，${count} 条`}
            className={`
              relative flex min-w-[76px] items-center justify-center gap-1 whitespace-nowrap rounded-lg px-1.5 py-1.5
              text-[12px] font-semibold transition-all duration-200
              ${
                isActive
                  ? "bg-white text-[#005aab] shadow-[0_1px_2px_rgba(31,41,55,0.10),0_0_0_1px_rgba(0,103,192,0.18)]"
                  : "text-slate-600 hover:bg-white/[0.62] hover:text-slate-900"
              }
            `}
          >
            <svg
              className={`h-3.5 w-3.5 ${isActive ? "text-[#0067c0]" : "text-slate-500"}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8} d={cat.icon} />
            </svg>
            <span>{cat.label}</span>
            <span
              aria-hidden={count === 0}
              className={`
                min-w-[16px] rounded-full px-1 py-0.5 text-center text-[10px] leading-none
                ${count === 0 ? "invisible" : ""}
                ${isActive ? "bg-[#0067c0]/10 text-[#005aab]" : "bg-slate-900/[0.06] text-slate-500"}
              `}
            >
              {count}
            </span>
          </button>
        );
      })}
    </div>
  );
};

export default CategoryTabs;
