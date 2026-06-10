import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipboardEntry } from "../types";

interface ClipboardItemProps {
  entry: ClipboardEntry;
  onDelete: (id: string) => void;
}

const ClipboardItem: React.FC<ClipboardItemProps> = ({ entry, onDelete }) => {
  const [hover, setHover] = useState(false);
  const [copied, setCopied] = useState(false);
  const [imageSrc, setImageSrc] = useState<string | null>(null);

  useEffect(() => {
    if (entry.contentType === "image" && entry.imagePath) {
      invoke<string>("read_image_file", { path: entry.imagePath })
        .then((data) => setImageSrc(`data:image/png;base64,${data}`))
        .catch(() => setImageSrc(null));
    }
  }, [entry.contentType, entry.imagePath]);

  const handleCopy = async () => {
    if (entry.contentType === "image") return;
    try {
      await navigator.clipboard.writeText(entry.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Fallback for some environments
    }
  };

  const typeStyles = {
    text: { bg: "bg-[#0067c0]/10", dot: "bg-[#0067c0]", text: "text-[#005aab]", label: "文本" },
    link: { bg: "bg-[#107c10]/10", dot: "bg-[#107c10]", text: "text-[#0b6a0b]", label: "链接" },
    image: { bg: "bg-[#8764b8]/10", dot: "bg-[#8764b8]", text: "text-[#6d4aa2]", label: "图片" },
  }[entry.contentType];

  const timeAgo = (() => {
    const created = new Date(entry.createdAt);
    const now = new Date();
    const diff = now.getTime() - created.getTime();
    const mins = Math.floor(diff / 60000);
    const hrs = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (mins < 1) return "刚刚";
    if (mins < 60) return `${mins}分钟前`;
    if (hrs < 24) return `${hrs}小时前`;
    if (days < 7) return `${days}天前`;
    return created.toLocaleDateString("zh-CN");
  })();

  return (
    <div
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onClick={handleCopy}
      className={`
        group relative mx-2 mb-2 cursor-pointer overflow-hidden rounded-xl
        border border-slate-900/[0.08] transition-all duration-200
        shadow-[0_1px_2px_rgba(31,41,55,0.05),0_1px_0_rgba(255,255,255,0.85)_inset]
        hover:-translate-y-0.5 hover:border-[#0067c0]/20 hover:shadow-[0_8px_24px_rgba(31,41,55,0.12),0_1px_0_rgba(255,255,255,0.92)_inset]
        ${hover ? "bg-white/[0.88]" : "bg-white/[0.62]"}
      `}
    >
      <div className="p-3">
        {/* Type dot + source app + time */}
        <div className="mb-2 flex items-center gap-2">
          <span className={`flex items-center gap-1.5 rounded-md px-1.5 py-0.5 ${typeStyles.bg}`}>
            <span className={`h-1.5 w-1.5 flex-shrink-0 rounded-full ${typeStyles.dot}`} />
            <span className={`text-[10px] font-semibold leading-none ${typeStyles.text}`}>
              {typeStyles.label}
            </span>
          </span>
          <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-slate-500">
            {entry.sourceApp || "未知应用"}
          </span>
          <span className="flex-shrink-0 text-[10px] font-medium tabular-nums text-slate-400">
            {timeAgo}
          </span>
        </div>

        {/* Content */}
        {entry.contentType === "image" ? (
          <div className="relative overflow-hidden rounded-lg border border-slate-900/[0.08] bg-slate-100">
            {imageSrc ? (
              <img
                src={imageSrc}
                alt={entry.preview}
                className="max-h-48 w-full rounded-lg object-cover"
                loading="lazy"
              />
            ) : (
              <div className="flex h-24 items-center justify-center text-xs font-medium text-slate-400">
                加载图片中...
              </div>
            )}
            <div className="absolute bottom-0 left-0 right-0 h-14 bg-gradient-to-t from-slate-950/55 to-transparent" />
            <span className="absolute bottom-2 left-2 max-w-[calc(100%-16px)] truncate text-[10px] font-medium text-white">
              {entry.preview}
            </span>
          </div>
        ) : (
          <p className="line-clamp-3 break-all text-[13px] leading-relaxed text-slate-800">
            {entry.preview}
          </p>
        )}
      </div>

      {/* Delete button — visible on hover */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onDelete(entry.id);
        }}
        className={`
          absolute right-2 top-2 rounded-lg border border-slate-900/[0.08] bg-white/[0.82] p-1.5
          text-slate-400 shadow-sm backdrop-blur-xl transition-all duration-150
          hover:border-[#c42b1c]/20 hover:bg-[#c42b1c]/10 hover:text-[#c42b1c]
          ${hover ? "opacity-100 scale-100" : "opacity-0 scale-90"}
        `}
        title="删除"
      >
        <svg className="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      {/* Copied indicator */}
      {copied && (
        <div className="absolute inset-0 flex animate-fade-in items-center justify-center bg-white/[0.82] backdrop-blur-xl">
          <span className="rounded-lg bg-[#107c10]/10 px-3 py-1.5 text-xs font-semibold text-[#107c10]">
            已复制
          </span>
        </div>
      )}
    </div>
  );
};

export default ClipboardItem;
