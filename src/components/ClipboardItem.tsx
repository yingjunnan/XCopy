import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
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

  const handlePreviewImage = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!entry.imagePath) return;

    // Pass the image path to the preview window via the URL hash (encoded),
    // so the preview page can read it standalone — no shared state needed.
    const encoded = encodeURIComponent(entry.imagePath);
    const url = `preview.html#${encoded}`;

    try {
      const existing = await WebviewWindow.getByLabel("image-preview");
      if (existing) {
        // Re-use the open preview window: focus it and tell it to load the new image.
        await existing.setFocus();
        await emit("preview-open-image", { path: entry.imagePath });
        return;
      }

      new WebviewWindow("image-preview", {
        url,
        title: "图片预览",
        width: 840,
        height: 620,
        minWidth: 360,
        minHeight: 280,
        resizable: true,
        decorations: true,
        center: true,
      });
    } catch (err) {
      console.error("Failed to open preview window:", err);
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
        shadow-[0_1px_2px_rgba(31,41,55,0.05)]
        hover:-translate-y-0.5 hover:border-[#0067c0]/20 hover:bg-white hover:shadow-[0_8px_20px_rgba(31,41,55,0.10)]
        ${hover ? "bg-white" : "bg-white"}
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

            {/* View large image — bottom-right */}
            <button
              onClick={handlePreviewImage}
              title="查看大图"
              aria-label="在新窗口查看大图"
              className={`
                no-drag absolute bottom-1.5 right-1.5 flex items-center gap-1 rounded-md
                bg-slate-950/55 px-1.5 py-1 text-[10px] font-semibold text-white backdrop-blur-sm
                transition-all duration-150 hover:bg-[#0067c0]/80 active:scale-95
                ${hover ? "translate-y-0 opacity-100" : "pointer-events-none translate-y-1 opacity-0"}
              `}
            >
              <svg className="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-4.35-4.35M11 8a3 3 0 11-6 0 3 3 0 016 0z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 3h4M3 3v4M21 3h-4M21 3v4M3 21h4M3 21v-4M21 21h-4M21 21v-4" />
              </svg>
              查看大图
            </button>
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
          absolute right-2 top-2 rounded-lg border border-slate-200 bg-white p-1.5
          text-slate-400 shadow-sm transition-all duration-150
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
        <div className="absolute inset-0 flex animate-fade-in items-center justify-center bg-white">
          <span className="rounded-lg bg-[#107c10]/10 px-3 py-1.5 text-xs font-semibold text-[#107c10]">
            已复制
          </span>
        </div>
      )}
    </div>
  );
};

export default ClipboardItem;
