# Onboarding（安装后引导界面）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 安装器完成后，应用首次启动时弹出一个独立的 3 步引导窗口，教用户使用 `Ctrl+Shift+V` 和双击 `Ctrl` 两个核心快捷键。

**Architecture:** 新建 standalone 的 `onboarding.html` + `src/onboarding.tsx`（参考现有 `quick_paste.html`/`src/quick_paste.tsx` 模式）。后端在 `lib.rs` setup 首启分支改为显示引导窗口而非主窗口，新增 `finish_onboarding` 命令在用户点"开始使用"时关闭引导并弹出主窗口。复用现有 `FIRST_RUN_MARKER` 机制。

**Tech Stack:** Tauri v2（Rust 后端）、React 18 + TypeScript、Tailwind CSS、Vite 多入口构建

**Spec:** `docs/superpowers/specs/2026-06-22-onboarding-design.md`

---

## File Structure

**新增**：
- `onboarding.html` — standalone HTML 入口，与 `quick_paste.html` 同构
- `src/onboarding.tsx` — 引导页面 React 组件，3 步状态机 + 键帽示意图

**修改**：
- `src-tauri/tauri.conf.json` — 新增 onboarding 窗口声明
- `src-tauri/capabilities/default.json` — windows 数组加 `onboarding`
- `src-tauri/src/lib.rs` — 首启分支改为显示引导窗口；新增 `show_onboarding_window` + `finish_onboarding`；注册命令
- `vite.config.ts` — 新增 onboarding 构建入口

---

## Task 1: 窗口声明与能力配置

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: 在 tauri.conf.json 的 app.windows 数组末尾新增 onboarding 窗口**

在 `src-tauri/tauri.conf.json` 的 `app.windows` 数组里，紧跟 `quick-paste` 窗口对象之后追加：

```json
,
{
  "label": "onboarding",
  "url": "onboarding.html",
  "title": "欢迎使用 XCopy",
  "width": 480,
  "height": 560,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "visible": false,
  "center": true,
  "shadow": true
}
```

- [ ] **Step 2: 在 capabilities/default.json 的 windows 数组加 "onboarding"**

将 `src-tauri/capabilities/default.json` 第 5 行：

```json
"windows": ["main", "image-preview", "quick-paste"],
```

改为：

```json
"windows": ["main", "image-preview", "quick-paste", "onboarding"],
```

现有权限（`core:window:allow-show`/`allow-hide`/`allow-close`/`allow-set-focus`）已覆盖引导窗口需求，无需新增权限。

- [ ] **Step 3: 验证配置可被 Tauri 加载**

Run: `npm run tauri build -- --debug --no-bundle 2>&1 | tail -20`（或仅 `cargo check --manifest-path src-tauri/Cargo.toml`）

预期：编译通过，无 schema 错误。若报 schema 错误，检查 JSON 逗号/括号。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat(onboarding): declare onboarding window and capability"
```

---

## Task 2: 构建入口配置

**Files:**
- Modify: `vite.config.ts`

- [ ] **Step 1: 在 vite.config.ts 的 rollupOptions.input 新增 onboarding 入口**

将 `vite.config.ts` 第 22-28 行的 `input` 对象：

```ts
      input: {
        main: resolve(__dirname, "index.html"),
        preview: resolve(__dirname, "preview.html"),
        quickPaste: resolve(__dirname, "quick_paste.html"),
      },
```

改为：

```ts
      input: {
        main: resolve(__dirname, "index.html"),
        preview: resolve(__dirname, "preview.html"),
        quickPaste: resolve(__dirname, "quick_paste.html"),
        onboarding: resolve(__dirname, "onboarding.html"),
      },
```

- [ ] **Step 2: Commit**

```bash
git add vite.config.ts
git commit -m "build(onboarding): add onboarding.html as vite entry"
```

（此时尚无 onboarding.html，构建会报错，下一任务创建文件后即可正常构建。先提交配置是独立的逻辑改动。）

---

## Task 3: 创建 onboarding.html 入口

**Files:**
- Create: `onboarding.html`

- [ ] **Step 1: 创建 onboarding.html，与 quick_paste.html 同构**

创建 `onboarding.html`，内容完全参照 `quick_paste.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>欢迎使用 XCopy</title>
    <style>
      body {
        margin: 0;
        padding: 0;
        overflow: hidden;
        background: transparent;
      }
    </style>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/onboarding.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: 验证构建配置可解析（无需完整构建）**

Run: `ls onboarding.html`

预期：文件存在。

- [ ] **Step 3: Commit**

```bash
git add onboarding.html
git commit -m "feat(onboarding): add onboarding.html entry"
```

---

## Task 4: 创建 onboarding.tsx 引导组件

**Files:**
- Create: `src/onboarding.tsx`

这是核心前端任务。3 步向导：欢迎 → Ctrl+Shift+V → 双击 Ctrl。纯展示，不试练。最后一步"开始使用"调 `finish_onboarding`。

- [ ] **Step 1: 创建 src/onboarding.tsx 完整实现**

创建 `src/onboarding.tsx`：

```tsx
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

type StepProps = {
  onNext: () => void;
  onPrev: () => void;
  isLast: boolean;
  isFirst: boolean;
};

const WelcomeStep: React.FC<StepProps> = ({ onNext }) => (
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

const ShortcutStep: React.FC<StepProps> = ({ onNext, onPrev, isLast }) => (
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

const DoubleCtrlStep: React.FC<StepProps> = ({ onPrev, isLast }) => {
  const finish = async () => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("finish_onboarding");
    } catch (err) {
      console.error("Failed to finish onboarding:", err);
    }
  };

  return (
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
};

const Onboarding: React.FC = () => {
  const [step, setStep] = useState(0);
  const total = 3;
  const steps: React.ReactElement[] = [
    <WelcomeStep
      key="welcome"
      onNext={() => setStep(1)}
      onPrev={() => {}}
      isFirst
      isLast={false}
    />,
    <ShortcutStep
      key="shortcut"
      onNext={() => setStep(2)}
      onPrev={() => setStep(0)}
      isFirst={false}
      isLast={false}
    />,
    <DoubleCtrlStep
      key="double-ctrl"
      onNext={() => {}}
      onPrev={() => setStep(1)}
      isFirst={false}
      isLast
    />,
  ];

  const current = steps[step];
  const isFirst = step === 0;
  const isLast = step === total - 1;

  const handlePrimary = () => {
    if (isLast) {
      // DoubleCtrlStep 内部按钮自己调 finish_onboarding
      // 这里是底部主按钮的逻辑
      // 但 DoubleCtrlStep 的完成动作通过 props 触发更清晰，
      // 为保持简单，最后一步的"开始使用"也调 finish_onboarding
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
```

**注意**：`DoubleCtrlStep` 内部定义了 `finish` 函数但未在组件内绑定（其"开始使用"动作由底部统一按钮 `handlePrimary` 触发）。`DoubleCtrlStep` 的 `finish` 局部函数可删除——上面保留是为了说明语义，实际由 `handlePrimary` 统一调用 `finish_onboarding`。**清理**：删除 `DoubleCtrlStep` 里未使用的 `finish` 函数定义，避免 lint 警告。

修正后的 `DoubleCtrlStep`：

```tsx
const DoubleCtrlStep: React.FC<StepProps> = ({ onPrev, isLast }) => (
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
```

- [ ] **Step 2: 验证 TypeScript 编译通过**

Run: `npx tsc --noEmit`

预期：无错误。若报 `isFirst`/`onPrev` 等未使用参数警告，确认 `StepProps` 类型定义里这些字段存在但不强制使用（React 函数组件 props 解构未使用的字段是允许的，tsc 默认不报错）。

- [ ] **Step 3: 验证前端构建通过**

Run: `npm run build`

预期：`dist/onboarding.html` 和对应 JS 资源生成。

- [ ] **Step 4: Commit**

```bash
git add src/onboarding.tsx
git commit -m "feat(onboarding): add 3-step onboarding React component"
```

---

## Task 5: 后端首启分支改为显示引导窗口

**Files:**
- Modify: `src-tauri/src/lib.rs`

此任务改 `lib.rs` setup 首启分支，并新增 `show_onboarding_window` 函数。**暂不**新增 `finish_onboarding` 命令（下一任务），这样本任务的改动可独立编译验证。

- [ ] **Step 1: 新增 show_onboarding_window 函数**

在 `src-tauri/src/lib.rs` 的 `show_main_window` 函数之后（约第 228 行后）新增：

```rust
fn show_onboarding_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        let _ = window.set_focus();
        eprintln!("[XCopy] onboarding window shown");
    } else {
        eprintln!("[XCopy] ERROR: onboarding window not found");
    }
}
```

- [ ] **Step 2: 修改 setup 首启分支**

在 `src-tauri/src/lib.rs` 的 setup 闭包里，找到首启分支（约第 353-363 行）：

```rust
            if !first_run_marker.exists() {
                eprintln!("[XCopy] first run detected, showing main window");
                show_main_window(app.handle(), false);
                if let Err(e) = std::fs::write(&first_run_marker, "") {
                    eprintln!(
                        "[XCopy] failed to write first-run marker at {}: {}",
                        first_run_marker.display(),
                        e
                    );
                }
            }
```

改为：

```rust
            if !first_run_marker.exists() {
                eprintln!("[XCopy] first run detected, showing onboarding");
                show_onboarding_window(app.handle());
                if let Err(e) = std::fs::write(&first_run_marker, "") {
                    eprintln!(
                        "[XCopy] failed to write first-run marker at {}: {}",
                        first_run_marker.display(),
                        e
                    );
                }
            }
```

- [ ] **Step 3: 验证 Rust 编译通过**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

预期：编译通过，无错误。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(onboarding): show onboarding window on first run instead of main"
```

---

## Task 6: 新增 finish_onboarding 命令

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 新增 finish_onboarding 命令函数**

在 `src-tauri/src/lib.rs` 的 `show_quick_paste_panel` 命令之后（约第 182 行后）新增：

```rust
#[tauri::command]
fn finish_onboarding(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.hide();
    show_main_window(&app, false);
    Ok(())
}
```

- [ ] **Step 2: 在 invoke_handler 注册 finish_onboarding**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 宏里（约第 367-379 行），找到：

```rust
        .invoke_handler(tauri::generate_handler![
            get_history,
            delete_entry,
            clear_history,
            get_last_entry,
            read_image_file,
            get_app_settings,
            save_app_settings,
            hide_main_window,
            get_storage_usage,
            show_quick_paste_panel,
            paste_from_quick_paste,
        ])
```

在 `paste_from_quick_paste,` 之后加一行 `finish_onboarding,`：

```rust
        .invoke_handler(tauri::generate_handler![
            get_history,
            delete_entry,
            clear_history,
            get_last_entry,
            read_image_file,
            get_app_settings,
            save_app_settings,
            hide_main_window,
            get_storage_usage,
            show_quick_paste_panel,
            paste_from_quick_paste,
            finish_onboarding,
        ])
```

- [ ] **Step 3: 验证 Rust 编译通过**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

预期：编译通过。

- [ ] **Step 4: 运行现有 Rust 测试确保无回归**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

预期：所有现有测试通过（`app_settings` 相关测试不受影响）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(onboarding): add finish_onboarding command to close guide and show main"
```

---

## Task 7: 集成手测验证

此任务无代码改动，是 spec 第 6.2 节的集成手测清单。Tauri 应用涉及窗口时序与全局快捷键，必须人工验证。

- [ ] **Step 1: 模拟首启环境**

删除首启 marker 模拟全新安装：

Run: `del "%APPDATA%\com.xcopy.app\.first_run_shown" 2>nul & echo done`

（若 app_data_dir 路径不同，用 `echo %APPDATA%\com.xcopy.app` 确认。）

- [ ] **Step 2: 启动应用并验证引导窗口弹出**

Run: `npm run tauri dev`

预期：
- 应用启动后，引导窗口弹出（480×560，居中，无边框圆角）
- 主窗口**不**弹出

- [ ] **Step 3: 验证 3 步向导翻页**

在引导窗口内：
- 点"下一步" → 进入第 2 步（Ctrl+Shift+V 键帽示意）
- 点"下一步" → 进入第 3 步（双击 Ctrl 键帽示意）
- 点"上一步" → 回到第 2 步
- 步骤指示器圆点随步骤变化

- [ ] **Step 4: 验证"开始使用"关闭引导并弹出主窗口**

在第 3 步点"开始使用"：
- 引导窗口关闭
- 主窗口弹出（空状态，因首启无历史）

- [ ] **Step 5: 验证重启后不再弹引导**

关闭应用，再次 `npm run tauri dev`：
- 引导窗口**不**弹出（marker 已写）
- 应用静默启动（托盘图标可见）

- [ ] **Step 6: 验证引导窗口不因失焦消失**

重启 `npm run tauri dev` 前先删 marker（重复 Step 1），引导弹出后点击桌面其他地方：
- 引导窗口**不**消失（区别于主窗口的失焦隐藏）

- [ ] **Step 7: 验证 Ctrl+Shift+V 在引导显示期间仍可用**

引导窗口显示时按 `Ctrl+Shift+V`：
- 主窗口弹出（全局快捷键不受引导影响）
- 引导窗口仍在

- [ ] **Step 8: 验证双击 Ctrl 在引导显示期间仍可用**

引导窗口显示时双击 Ctrl：
- quick-paste 面板在鼠标位置弹出（低级钩子不受引导影响）

- [ ] **Step 9: 验证强制关引导不弹主窗口**

删 marker 重启，引导弹出后按 Alt+F4（若无边框窗口响应）或任务管理器结束 XCopy 进程的引导窗口：
- 若 Alt+F4 有效：引导关闭，主窗口**不**弹出
- marker 已写，重启不重弹引导

- [ ] **Step 10: 记录手测结果**

在手测清单上勾选通过项。若任何步骤失败，回到对应任务修复后重测。

---

## Self-Review

**1. Spec coverage（逐节核对）：**

- Spec 2 节"核心交互流程" → Task 5（首启分支）+ Task 6（finish_onboarding）✅
- Spec 3 节"架构与组件" 4.1 后端 → Task 5 + Task 6 ✅
- Spec 4.2 前端 → Task 4 ✅
- Spec 4.3 窗口配置 → Task 1 ✅
- Spec 4.4 能力配置 → Task 1 ✅
- Spec 4.5 构建配置 → Task 2 ✅
- Spec 4.6 数据流 → Task 5 + Task 6 + Task 4 ✅
- Spec 5 节"边界与错误处理" → Task 7 手测覆盖（失焦、强制关闭、marker、重启）✅
- Spec 6 节"测试" 6.2 集成手测 → Task 7 ✅
- Spec 6.1 Rust 单测 → spec 明确"无需新单测"，无对应任务 ✅
- Spec 8 节"受影响文件清单" → 所有文件均被任务覆盖 ✅

**2. Placeholder scan：**
- 无 TBD/TODO/"implement later" ✅
- 所有代码步骤都有完整代码 ✅
- Task 4 的 `DoubleCtrlStep` 已修正删除未使用的 `finish` 函数 ✅
- Task 7 手测步骤都有明确命令与预期 ✅

**3. Type consistency：**
- `show_onboarding_window(app: &tauri::AppHandle)` — Task 5 定义，Task 5 setup 调用，签名一致 ✅
- `finish_onboarding(app: tauri::AppHandle, window: tauri::WebviewWindow)` — Task 6 定义，Task 4 前端 `invoke("finish_onboarding")` 调用，命令名一致 ✅
- `show_main_window(&app, false)` — Task 6 调用，与 lib.rs 现有签名 `fn show_main_window(app: &AppHandle, capture_clipboard: bool)` 一致 ✅
- onboarding 窗口 label `"onboarding"` — Task 1 声明、Task 5 `get_webview_window("onboarding")`、Task 4 前端一致 ✅
- `FIRST_RUN_MARKER` 常量 — Task 5 复用现有常量，未改名 ✅

无问题。
