# 安装后引导界面（Onboarding）设计

- 日期：2026-06-22
- 分支：`feat/onboarding`（待创建）
- 状态：待实现

## 1. 目标

安装器完成后，应用首次启动时弹出一个独立的引导窗口，用 3 步向导教用户如何使用两个核心快捷键：

1. `Ctrl+Shift+V` 唤起主窗口（剪贴板历史）
2. 双击 `Ctrl` 唤起快速粘贴面板（直接粘贴到光标）

让新用户在 30 秒内知道"怎么呼出界面"，避免装完不知道怎么用。

## 2. 核心交互流程

```
安装器结束
 → 用户勾选“启动 XCopy”（Tauri NSIS 默认勾选框，不改安装器）
 → XCopy.exe 启动
 → setup 阶段检测 FIRST_RUN_MARKER 不存在
 → 写入 marker（防止引导崩溃导致每次启动都弹）
 → 显示引导窗口（而非主窗口）
 → 用户翻阅 3 步向导
 → 点“开始使用”
 → 关闭引导窗口 + 显示主窗口（show_main_window）
 → 后续启动 marker 存在，正常隐藏启动
```

## 3. 关键决策（已与用户确认）

| 决策点 | 选择 | 备注 |
|--------|------|------|
| 触发时机 | 安装器结束后自动启动应用，首启弹引导 | 复用现有 NSIS 默认"启动应用"勾选框，不改 installer_hooks.nsh |
| 引导形态 | 独立专用引导窗口 | standalone HTML，参考 preview.html / quick_paste.html 模式 |
| 引导步骤 | 3 步向导：欢迎 / Ctrl+Shift+V / 双击 Ctrl | 纯展示，不含试练交互 |
| 试练交互 | 不试练 | 只展示示意图和说明，不检测用户是否真按了快捷键 |
| 触发频率 | 仅首启弹一次 | 复用 FIRST_RUN_MARKER，写后永不重弹 |
| 引导完成后行为 | 关闭引导窗口 + 弹出主窗口 | 用户点"开始使用"后立即看到主界面（方案 B） |
| 引导窗口尺寸 | 480×560 | 比主窗口（420×640）略宽略矮，更像向导 |
| marker 写时机 | 引导窗口一打开就写 | 防止引导崩溃导致每次启动都弹 |
| 强制关引导 | 允许关闭，不弹主窗口 | 用户主动放弃引导，marker 已写，重启不重弹 |

## 4. 架构与组件

### 4.1 后端（Rust）

**`src-tauri/src/lib.rs` 改动**：

1. 首启分支从"显示主窗口 + 写 marker"改为"显示引导窗口 + 写 marker"：

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
// 注意：不再在首启时调用 show_main_window
```

2. 新增函数 `show_onboarding_window`：

```rust
fn show_onboarding_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        eprintln!("[XCopy] ERROR: onboarding window not found");
    }
}
```

3. 新增 Tauri 命令 `finish_onboarding`，引导窗口"开始使用"按钮调用：

```rust
#[tauri::command]
fn finish_onboarding(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.hide();
    show_main_window(&app, false);
    Ok(())
}
```

- `show_main_window(app, false)`：`capture_clipboard=false`，首启无历史无需捕获。
- `show_main_window` 现有实现会 `set_skip_taskbar(false)` + `show` + `set_focus` + `request_history_refresh`，首启调用安全（空历史走空状态）。

4. `invoke_handler` 注册 `finish_onboarding`。

5. `on_window_event` 对 `onboarding` label **不做**失焦隐藏处理（引导不应因点别处就消失）。现有逻辑只对 `main` 和 `quick-paste` label 做失焦处理，`onboarding` 自然不在其中，无需改动该分支。但需确认 `onboarding` 的 `CloseRequested` 行为：允许关闭，不弹主窗口（用户主动放弃），marker 已写。

### 4.2 前端（React）

**新增 `src/onboarding.tsx` + `onboarding.html`**（standalone，参考 `preview.tsx`/`quick_paste.tsx` 模式）。

3 步向导，单页应用，`useState` 管理当前步骤（0/1/2）：

| 步骤 | 标题 | 内容 | 示意图 |
|------|------|------|--------|
| 0 — 欢迎 | 欢迎使用 XCopy | 简短介绍：自动记录剪贴板，按快捷键唤起 | XCopy logo + 一句话定位 |
| 1 — 主窗口快捷键 | 唤出剪贴板历史 | 按 `Ctrl+Shift+V` 弹出主窗口，选条目即复制回剪贴板 | 键帽动画：Ctrl+Shift+V 高亮 + 主窗口示意 |
| 2 — 快速粘贴 | 双击 Ctrl，秒粘贴 | 在任何应用打字时双击 Ctrl，选条目直接粘贴到光标 | 双击 Ctrl 键帽动画 + 粘贴示意 |

**UI 组件结构**：
- 顶部：步骤指示器（3 个圆点，当前步高亮）
- 中部：每步的内容区（标题 + 说明 + 键盘示意图，用 CSS/SVG 键帽）
- 底部：`上一步` / `下一步` / `开始使用`（仅最后一步）按钮

**键帽示意图**：用纯 CSS 实现 `<kbd>` 风格键帽（参考 `landing.html` 里 `.kbd` 样式），带 `pulse` 动画提示按压。双击 Ctrl 步骤用两次 Ctrl 键帽 + 连接线示意"快速连按"。

**完成动作**：最后一步"开始使用"按钮调用 `invoke('finish_onboarding')`。

**样式**：与主窗口品牌一致（白底、圆角、`#0067c0` 主色），无边框窗口 `rounded-[18px]` + ring 边框，参考 `App.tsx` 的容器样式。

### 4.3 窗口配置

**`src-tauri/tauri.conf.json` `app.windows` 新增**：

```json
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

比主窗口略小、居中、无边框圆角，符合"向导"气质。`visible: false`，由 `show_onboarding_window` 在首启时 show。

### 4.4 能力配置

**`src-tauri/capabilities/default.json`**：
- `windows` 数组加 `"onboarding"`
- 现有权限（`core:window:allow-show`/`allow-hide`/`allow-close`/`allow-set-focus`）已覆盖引导窗口需求，无需新增权限

### 4.5 构建配置

**`vite.config.ts`**：在 `rollupOptions.input` 新增 `onboarding` 入口：

```ts
input: {
  main: resolve(__dirname, "index.html"),
  preview: resolve(__dirname, "preview.html"),
  quickPaste: resolve(__dirname, "quick_paste.html"),
  onboarding: resolve(__dirname, "onboarding.html"),
},
```

### 4.6 数据流

```
setup 检测 marker 缺失
 → show_onboarding_window(app)  // 显示引导窗口
 → 写 marker
 → 用户在引导窗口翻页（纯前端状态）
 → 点“开始使用”
 → invoke('finish_onboarding')
 → Rust: 隐藏引导窗口 + show_main_window(app, false)
 → 主窗口显示（空状态），引导窗口隐藏
 → 后续启动 marker 存在，setup 不触发引导
```

## 5. 边界与错误处理

- **marker 写入失败**：现有逻辑已处理（`eprintln!` 日志）。即使失败，下次启动可能重弹引导——可接受，不致命。
- **引导窗口未找到**：`show_onboarding_window` 里 `get_webview_window` 返回 None 时静默降级（`eprintln!` 日志），不阻塞应用启动，仍写 marker。
- **用户强制关引导窗口（Alt+F4/任务管理器）**：允许关闭，不调 `show_main_window`（用户主动放弃）。marker 已写，下次不重弹。无边框窗口无关闭按钮，但系统级关闭（Alt+F4）仍可能触发；`on_window_event` 不为 onboarding 拦截 `CloseRequested`，允许默认关闭。
- **多显示器**：`center: true` 由 Tauri 处理，居中到主屏。
- **失焦行为**：引导窗口不走主窗口的失焦隐藏逻辑。`on_window_event` 里只对 `main` 和 `quick-paste` label 做失焦处理，`onboarding` 不处理（引导不应因点别处就消失）。
- **静默安装（/S）**：静默安装不启动应用，marker 不写，下次用户手动启动仍弹引导——符合预期。
- **卸载重装是否重弹引导**：现有卸载器（`NSIS_HOOK_POSTUNINSTALL`）只删 Run key，不删 app_data_dir。因此卸载重装后 marker 仍存在，引导不重弹。**此为首版可接受行为**（与"仅首启弹一次"决策一致；用户若想重看引导，需手动删 app_data_dir 下的 `.first_run_shown`）。实现阶段验证此行为，文档中注明。

## 6. 测试

### 6.1 Rust 单测

无新增纯函数逻辑（`show_onboarding_window` 和 `finish_onboarding` 是薄封装，依赖 Tauri 运行时），无需新单测。现有 `app_settings` 测试不变。

### 6.2 集成手测（必须，涉及 UI 时序）

- 全新安装（先删 app_data_dir 模拟首启）→ 启动应用 → 引导窗口弹出，主窗口不弹
- 引导 3 步翻页正常，键帽动画显示
- 点"开始使用" → 引导关闭 + 主窗口弹出（空状态）
- 关闭应用重启 → marker 存在 → 引导不再弹，应用静默启动
- 引导窗口点外面 → 不消失（区别于主窗口）
- 强制关引导（Alt+F4）→ 主窗口不弹，marker 已写，重启不重弹
- 卸载重装 → 验证 marker 是否仍存在（预期：仍存在，引导不重弹）
- 静默安装（/S）→ 不启动应用，手动启动后弹引导

## 7. 不在范围内（YAGNI）

- 试练交互（检测用户真按了快捷键才进下一步）
- 引导内可跳过/可重看（设置页"重新查看引导"按钮）
- 引导内展示托盘/开机自启说明（仅 3 步：欢迎/快捷键/双击 Ctrl）
- 多语言（首版中文，与现有 UI 一致）
- 引导内嵌入真实主窗口预览（用 CSS 示意图即可）
- NSIS 安装器内嵌向导页
- 卸载时清除 marker（首版接受重装不重弹）

## 8. 受影响文件清单

**新增**：
- `src/onboarding.tsx`（引导页面 React 组件，3 步状态机）
- `onboarding.html`（standalone HTML 入口）

**修改**：
- `src-tauri/src/lib.rs`（首启分支改为显示引导窗口；新增 `show_onboarding_window` 函数 + `finish_onboarding` 命令；`invoke_handler` 注册新命令）
- `src-tauri/tauri.conf.json`（新增 onboarding 窗口声明）
- `src-tauri/capabilities/default.json`（onboarding 窗口加入 windows 数组）
- `vite.config.ts`（新增 onboarding.html 构建入口）
