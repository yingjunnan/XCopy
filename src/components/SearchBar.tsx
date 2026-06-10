import React from "react";

interface SearchBarProps {
  value: string;
  onChange: (val: string) => void;
}

const SearchBar: React.FC<SearchBarProps> = ({ value, onChange }) => {
  return (
    <div className="no-drag relative px-4">
      <div
        className="
          flex items-center gap-2.5 rounded-xl border border-slate-900/[0.10]
          bg-white px-3.5 py-2.5
          shadow-[0_1px_2px_rgba(31,41,55,0.06)]
          transition-all duration-200
          focus-within:border-[#0067c0]/50
          focus-within:shadow-[0_0_0_3px_rgba(0,103,192,0.12),0_1px_2px_rgba(31,41,55,0.06)]
        "
      >
        <svg
          className="h-4 w-4 flex-shrink-0 text-slate-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="搜索剪切板内容或来源应用..."
          className="
            w-full bg-transparent font-sans text-[13px] leading-5 text-slate-900
            outline-none placeholder:text-slate-400
          "
        />
        {value && (
          <button
            onClick={() => onChange("")}
            className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md text-slate-400 transition-colors hover:bg-slate-900/[0.06] hover:text-slate-700"
            title="清除搜索"
          >
            <svg className="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>
    </div>
  );
};

export default SearchBar;
