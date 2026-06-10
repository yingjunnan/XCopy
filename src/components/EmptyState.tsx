import React from "react";

const EmptyState: React.FC = () => {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 text-center px-8">
      <div className="relative">
        <div className="w-20 h-20 rounded-2xl bg-white/[0.03] border border-white/[0.06] flex items-center justify-center">
          <svg
            className="w-8 h-8 text-white/15"
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
        <div className="absolute -top-1 -right-1 w-4 h-4 rounded-full border-2 border-[#12121f] bg-amber-500/80" />
      </div>
      <div>
        <p className="text-sm text-white/40 font-medium mb-1">剪贴板为空</p>
        <p className="text-xs text-white/20 leading-relaxed">
          使用 Ctrl+C 复制内容，然后按 Ctrl+Shift+V 查看
        </p>
      </div>
    </div>
  );
};

export default EmptyState;
