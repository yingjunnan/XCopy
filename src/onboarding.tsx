import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./index.css";

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** 键帽组件：圆角方块 + 内阴影，模拟物理按键。 */
const KeyCap: React.FC<{ children: React.ReactNode; pulse?: boolean }> = ({
  children,
  pulse,
}) => (
  <span
    className={`
      inline-flex min-w-[2.4rem] items-center justify-center rounded-lg
      border border-slate-300 bg-white px-3 py-2
      font-mono text-[14px] font-semibold text-slate-700 shadow-[0_2px_0_rgba(15,23,42,0.15)]
      ${pulse ? "animate-pulse ring-2 ring-[#0067c0]/40" : ""}
    `}
  >
    {children}
  </span>
);

/** 步骤指示器：3 个圆点，当前步高亮。 */
const Stepper: React.FC<{ current: number; total: number }> = ({
  current,
  total,
}) => (
  <div className="flex items-center justify-center gap-2">
    {Array.from({ length: total }).map((_, i) => (
      <span
        key={i}
        className={`
          h-1.5 rounded-full transition-all duration-200
          ${i === current ? "w-6 bg-[#0067c0]" : "w-1.5 bg-slate-300"}
        `}
      />
    ))}
  </div>
);

const WelcomeStep: React.FC = () => (
  <div className="flex h-full flex-col items-center justify-center px-8 text-center">
    <img
      src="/xcopy.png"
      alt="XCopy"
      draggable={false}
      className="mb-6 h-16 w-16 select-none rounded-[12px] object-cover"
    />
    <h1 className="mb-3 text-[22px] font-bold text-slate-800">
      欢迎使用 XCopy
    </h1>
    <p className="mb-2 text-[14px] leading-6 text-slate-500">
      轻量的 Windows 剪贴板历史工具
    </p>
    <p className="text-[13px] leading-6 text-slate-400">
      自动记录文本、链接与图片，按下快捷键即可呼出。
    </p>
  </div>
);

const ShortcutStep: React.FC = () => (
  <div className="flex h-full flex-col px-8 py-6">
    <div className="mb-1 text-[11px] font-semibold uppercase tracking-wider text-[#0067c0]">
      第 2 步
    </div>
    <h2 className="mb-3 text-[20px] font-bold text-slate-800">
      唤出剪贴板历史
    </h2>
    <p className="mb-6 text-[13px] leading-6 text-slate-500">
      在任意应用中按下快捷键，主窗口会即时弹出。选择一条记录即可复制回剪贴板，失焦自动隐藏。
    </p>

    <div className="mb-6 flex flex-1 items-center justify-center">
      <div className="flex items-center gap-3">
        <KeyCap pulse>Ctrl</KeyCap>
        <span className="text-[16px] font-bold text-slate-400">+</span>
        <KeyCap pulse>Shift</KeyCap>
        <span className="text-[16px] font-bold text-slate-400">+</span>
        <KeyCap pulse>V</KeyCap>
      </div>
    </div>

    <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-[12px] text-slate-500">
      提示：选完条目后，直接按 <span className="font-mono font-semibold">Ctrl+V</span> 粘贴到光标处。
    </div>
  </div>
);

const DoubleCtrlStep: React.FC = () => (
  <div className="flex h-full flex-col px-8 py-6">
    <div className="mb-1 text-[11px] font-semibold uppercase tracking-wider text-[#0067c0]">
      第 3 步
    </div>
    <h2 className="mb-3 text-[20px] font-bold text-slate-800">
      双击 Ctrl，秒粘贴
    </h2>
    <p className="mb-6 text-[13px] leading-6 text-slate-500">
      打字时连续快速按两次 Ctrl，会在鼠标位置弹出轻量面板。选中一条，内容直接粘贴到当前光标，无需再按 Ctrl+V。
    </p>

    <div className="mb-6 flex flex-1 items-center justify-center">
      <div className="flex items-center gap-2">
        <KeyCap pulse>Ctrl</KeyCap>
        <span className="text-[13px] font-semibold text-slate-400">快速连按</span>
        <KeyCap pulse>Ctrl</KeyCap>
      </div>
    </div>

    <div className="rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-[12px] text-slate-500">
      面板仅显示文本与链接，选中即粘贴。Esc 或点外面关闭。
    </div>
  </div>
);

const Onboarding: React.FC = () => {
  const [step, setStep] = useState(0);
  const total = 3;
  const steps: React.ReactElement[] = [
    <WelcomeStep key="welcome" />,
    <ShortcutStep key="shortcut" />,
    <DoubleCtrlStep key="double-ctrl" />,
  ];

  const current = steps[step];
  const isFirst = step === 0;
  const isLast = step === total - 1;

  const handlePrimary = () => {
    if (isLast) {
      if (isTauriRuntime()) {
        invoke("finish_onboarding").catch((err) =>
          console.error("Failed to finish onboarding:", err),
        );
      }
    } else {
      setStep((s) => Math.min(s + 1, total - 1));
    }
  };

  return (
    <div
      onContextMenu={(e) => e.preventDefault()}
      className="relative flex h-full w-full flex-col overflow-hidden rounded-[18px] bg-white"
    >
      <div className="pointer-events-none absolute inset-0 z-20 rounded-[18px] ring-1 ring-inset ring-slate-300" />

      {/* 顶部步骤指示器 */}
      <div className="flex h-12 flex-shrink-0 items-center justify-center">
        <Stepper current={step} total={total} />
      </div>

      {/* 内容区 */}
      <div className="relative z-10 flex-1 overflow-hidden">{current}</div>

      {/* 底部按钮 */}
      <div className="flex h-16 flex-shrink-0 items-center justify-between px-8">
        {!isFirst ? (
          <button
            type="button"
            onClick={() => setStep((s) => Math.max(s - 1, 0))}
            className="rounded-lg px-4 py-2 text-[13px] font-medium text-slate-500 transition hover:bg-slate-100"
          >
            上一步
          </button>
        ) : (
          <span />
        )}

        <button
          type="button"
          onClick={handlePrimary}
          className="rounded-lg bg-[#0067c0] px-6 py-2 text-[13px] font-semibold text-white shadow-[0_4px_12px_rgba(0,103,192,0.25)] transition hover:bg-[#005aab] active:scale-95"
        >
          {isLast ? "开始使用" : "下一步"}
        </button>
      </div>
    </div>
  );
};

createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Onboarding />
  </React.StrictMode>,
);
