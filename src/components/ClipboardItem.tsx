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
    text: { bg: "bg-blue-500/10", dot: "bg-blue-400", label: "文本" },
    link: { bg: "bg-emerald-500/10", dot: "bg-emerald-400", label: "链接" },
    image: { bg: "bg-purple-500/10", dot: "bg-purple-400", label: "图片" },
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
        group relative mx-2 mb-1 rounded-xl overflow-hidden cursor-pointer
        border border-white/[0.04] transition-all duration-200
        hover:border-white/[0.08] hover:bg-white/[0.03]
        ${hover ? "bg-white/[0.03]" : "bg-transparent"}
      `}
    >
      <div className="p-3">
        {/* Type dot + source app + time */}
        <div className="flex items-center gap-2 mb-1.5">
          <div className={`w-1.5 h-1.5 rounded-full ${typeStyles.dot} flex-shrink-0`} />
          <span className="text-[11px] text-white/25 font-mono truncate flex-1">
            {entry.sourceApp || "未知应用"}
          </span>
          <span className="text-[10px] text-white/15 flex-shrink-0 tabular-nums">
            {timeAgo}
          </span>
        </div>

        {/* Content */}
        {entry.contentType === "image" ? (
          <div className="relative rounded-lg overflow-hidden bg-white/[0.02]">
            {imageSrc ? (
              <img
                src={imageSrc}
                alt={entry.preview}
                className="w-full max-h-48 object-cover rounded-lg"
                loading="lazy"
              />
            ) : (
              <div className="flex items-center justify-center h-24 text-white/20 text-xs">
                加载图片中...
              </div>
            )}
            <div className="absolute bottom-0 left-0 right-0 h-12 bg-gradient-to-t from-black/60 to-transparent" />
            <span className="absolute bottom-2 left-2 text-[10px] text-white/60">
              {entry.preview}
            </span>
          </div>
        ) : (
          <p className="text-sm text-white/75 leading-relaxed line-clamp-3 break-all">
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
          absolute top-2 right-2 p-1.5 rounded-lg
          bg-black/40 hover:bg-red-500/20 text-white/40 hover:text-red-400
          transition-all duration-150
          ${hover ? "opacity-100 scale-100" : "opacity-0 scale-90"}
        `}
        title="删除"
      >
        <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>

      {/* Copied indicator */}
      {copied && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-fade-in">
          <span className="text-xs text-emerald-400 font-medium">已复制</span>
        </div>
      )}
    </div>
  );
};

export default ClipboardItem;
