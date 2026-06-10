import React from "react";

interface SearchBarProps {
  value: string;
  onChange: (val: string) => void;
}

const SearchBar: React.FC<SearchBarProps> = ({ value, onChange }) => {
  return (
    <div className="relative px-3 no-drag">
      <div
        className="
          flex items-center gap-2 px-3 py-2 rounded-xl
          bg-white/[0.04] border border-white/[0.06]
          focus-within:border-white/[0.12] focus-within:bg-white/[0.06]
          transition-all duration-200
        "
      >
        <svg
          className="w-3.5 h-3.5 text-white/30 flex-shrink-0"
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
            w-full bg-transparent text-white/90 text-sm
            placeholder:text-white/25 outline-none
            font-sans
          "
        />
        {value && (
          <button
            onClick={() => onChange("")}
            className="text-white/30 hover:text-white/60 transition-colors flex-shrink-0"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>
    </div>
  );
};

export default SearchBar;
