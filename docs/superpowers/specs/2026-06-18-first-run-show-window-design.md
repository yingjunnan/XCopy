# 首次启动弹出主窗口 — 设计文档

## 背景

XCopy 是一个 Tauri 剪贴板管理器。主窗口在 `tauri.conf.json` 中配置为 `visible: false`，
启动后默认隐藏，仅通过全局快捷键（默认 `Ctrl+Shift+V`）或托盘点击触发 `show_main_window` 显示。

用户希望在**安装后第一次启动**时自动弹出主窗口，让新用户感知应用已就绪；之后的启动恢复
原有的隐藏行为。

## 目标

- 安装后首次启动：主窗口自动弹出一次。
- 后续启动：保持隐藏，沿用快捷键/托盘唤出（现状不变）。
- 不新增设置项、不改动 `tauri.conf.json` 的 `visible: false`。

## 非目标

- 不做“开机自启时也弹出”的逻辑区分（开机自启属于第一次启动之外的常规启动，保持隐藏）。
- 不与 NSIS 安装器联动（不传 `--first-run` 参数）。

## 方案：标记文件（Approach A）

在 app data 目录下用一个标记文件记录“是否已展示过首次启动窗口”。

### 检测逻辑

在 `src-tauri/src/lib.rs` 的 `setup` 闭包中，**在现有初始化（DB、剪贴板监控、快捷键、托盘）
完成之后**，执行首次启动检查：

1. 定义常量 `FIRST_RUN_MARKER: &str = ".first_run_shown";`，路径为
   `app_data_dir.join(FIRST_RUN_MARKER)`。
2. 若标记文件**不存在** → 视为首次启动：
   - 调用 `show_main_window(app.handle(), false)` 弹出窗口（`false` 表示不抓取剪贴板，
     首次启动无有意义内容可抓取；该函数已负责 `show()` + `set_focus()` + 历史刷新）。
   - 随后创建标记文件（best-effort；失败仅 `eprintln!` 记录，不中断）。
3. 若标记文件**已存在** → 不做任何事，窗口保持隐藏。

### 为何用标记文件而非复用 settings.json

`settings.json` 只在用户手动“保存设置”时写入。从不开设置面板的用户会让该文件长期缺失，
若以“settings.json 缺失”判定首次启动，会导致每次启动都误判为首次——与“仅首次弹出”相悖。
独立标记文件在第一次展示后立即写入，判定可靠，且不依赖用户行为。

### 窗口行为

首次弹出的窗口沿用现有 `on_window_event` 的失焦自动隐藏逻辑（150ms 后若未获焦则 `hide()`），
即“正常弹出”行为，与快捷键唤出一致。

## 错误处理

- 标记文件写入失败：非致命，记录日志后继续。最坏情况是下次启动再次弹出，无害。
- `show_main_window` 内部失败已自行 `eprintln!` 记录，不影响主流程。

## 受影响文件

- `src-tauri/src/lib.rs`：新增 `FIRST_RUN_MARKER` 常量及 `setup` 中的首次启动检查。
  其余文件（`tauri.conf.json`、`app_settings.rs`、前端）均不改动。

## 测试与验证

- 单元测试不适用（涉及真实 app data 目录与窗口系统）。
- 手动验证：
  1. 删除 `%APPDATA%\com.xcopy.app\.first_run_shown`（或整个目录）→ 启动 → 主窗口出现。
  2. 再次启动 → 主窗口保持隐藏，快捷键可正常唤出。
- 现有 `cargo test`（`app_settings` 相关）保持通过，逻辑未变。
