import React from "react";

const EmptyState: React.FC = () => {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 px-8 text-center">
      <div className="relative">
        <div className="flex h-20 w-20 items-center justify-center rounded-2xl border border-slate-200 bg-white shadow-[0_8px_20px_rgba(31,41,55,0.08)]">
          <svg
            className="h-8 w-8 text-[#0067c0]/65"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
            />
          </svg>
        </div>
        <div className="absolute -right-1 -top-1 h-4 w-4 rounded-full border-2 border-white bg-[#f9a825]" />
      </div>
      <div>
        <p className="mb-1 text-sm font-semibold text-slate-700">剪贴板为空</p>
        <p className="text-xs leading-relaxed text-slate-500">
          使用 Ctrl+C 复制内容，然后按 Ctrl+Shift+V 查看
        </p>
      </div>
    </div>
  );
};

export default EmptyState;
