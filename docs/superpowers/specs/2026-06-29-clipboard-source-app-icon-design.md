# 剪贴板历史应用图标 设计文档

**日期**: 2026-06-29
**状态**: 已确认，待实现

## 背景

剪贴板历史记录每条消息来自哪个应用，但目前 `source_app` 字段存的是前台窗口标题（通过 `GetWindowTextW`，如 "剪贴板历史 - Google Chrome"），并非真正的应用名，且没有任何图标展示。前端在 `ClipboardItem.tsx` 里把这个标题当 "应用名" 文本显示。

本设计为每条剪贴板记录补充来源应用的真实图标，并把显示的应用名从窗口标题改为进程的产品名。

## 目标

- 每条剪贴板记录展示来源应用的图标。
- "应用名" 文字从窗口标题改为进程的产品名（如 "Google Chrome"），更准确统一。
- 图标提取对剪贴板捕获流程零阻塞：任何一步失败都不影响记录写入。

## 非目标

- 不回溯填充旧记录的图标与产品名（旧记录 `source_app` 仍是窗口标题、`source_app_icon` 为 NULL）。
- 不改动 quick-paste 面板（它用单色圆点 + preview，不显示 sourceApp）。
- 不做图标内存缓存层（磁盘缓存已足够；进程内重复提取同一 exe 会被磁盘缓存命中跳过）。

## 确认的决策

| 决策点 | 选择 |
|--------|------|
| 应用名来源 | 进程产品名（`SHGFI_DISPLAYNAME`），不再用窗口标题 |
| 图标持久化 | 磁盘缓存为 PNG + DB 存缓存路径 |
| 图标样式 | A 方案：应用名文字左侧，16px，4px 圆角，`object-fit: cover`；类型彩色小标签不变 |
| 图标提取尺寸 | 32×32（前端 CSS 缩到 16px 显示，retina 高分屏仍清晰） |

## 架构

### 数据流

```
剪贴板捕获 (clipboard.rs capture_from_clipboard)
  │
  ├─ window_tracker::win::get_source_app_info()   ← 新增，替代 get_active_window_title()
  │     返回 SourceAppInfo { name, icon_path: Option<PathBuf> }
  │     │
  │     ├─ HWND → PID (GetWindowThreadProcessId)
  │     ├─ PID → exe 路径 (OpenProcess + QueryFullProcessImageNameW)
  │     ├─ name = SHGetFileInfoW(exe, SHGFI_DISPLAYNAME)  失败→窗口标题 fallback
  │     └─ icon_path = 缓存命中则复用；否则 SHGetFileInfoW(exe, SHGFI_ICON|SHGFI_LARGEICON)
  │                    → HICON → RGBA → rgba_to_png → 写 app_icons/<hash>.png
  │
  ├─ 写入 ClipboardEntry { source_app: name, source_app_icon: icon_path }
  └─ db.insert_entry_if_changed(entry)
```

### 组件划分

1. **`window_tracker.rs` (win 模块)** — 获取来源应用信息
   - 新增结构 `SourceAppInfo { name: String, icon_path: Option<PathBuf> }`
   - 新增 `get_source_app_info(app_data_dir: &Path) -> SourceAppInfo`
   - 保留旧 `get_active_window_title()` 作为产品名失败的 fallback（内部仍可用）。
   - 图标提取的 PNG 编码复用 `clipboard.rs` 现有 `rgba_to_png / write_png_chunk / crc32 / deflate`——需把这些函数提升为模块级/可被 window_tracker 调用（例如抽到一个 `png_encode` 子模块或在 clipboard.rs 里 `pub(crate)`）。

2. **`clipboard.rs`** — 调用方
   - `capture_from_clipboard` 里把 `let source_app = win::get_active_window_title();` 改为调用 `win::get_source_app_info(&app_data_dir)`，把 `name` 与 `icon_path` 填进两个 `ClipboardEntry` 构造点（文本与图片分支）。
   - PNG 编码相关函数调整为可共享。

3. **`models.rs`** — 数据模型
   - `ClipboardEntry` 新增 `pub source_app_icon: Option<String>`（序列化为 `sourceAppIcon`）。

4. **`db.rs`** — 持久化
   - 建表 SQL 加列 `source_app_icon TEXT`。
   - 轻量迁移：`Database::new` 建表后，检测 `clipboard_history` 是否有 `source_app_icon` 列（`PRAGMA table_info`），无则 `ALTER TABLE clipboard_history ADD COLUMN source_app_icon TEXT`。这样老用户升级不丢数据。
   - `insert_entry` / `insert_entry_if_changed`(经由 insert_entry) / `query_entries` / `get_last_entry` 的 SQL 全部补上 `source_app_icon`。
   - `prune_history` 删除条目时不需额外清理图标文件（图标按 exe hash 共享缓存，可能被多条记录或多应用共享，不能随单条删除）。`clear_all` 也**不**删 `app_icons/` 目录（只清 `images/`，与现状一致）。

5. **前端**
   - `src/types/index.ts`：`ClipboardEntry` 加 `sourceAppIcon?: string | null;`。
   - `src/components/ClipboardItem.tsx`：在应用名文字左侧加图标：
     - 新增 state `iconSrc`，`useEffect` 里当 `entry.sourceAppIcon` 存在时调用现有 `invoke<string>("read_image_file", { path })` 取 base64，与图片预览同套机制。
     - 渲染 `<img src={iconSrc} className="h-4 w-4 flex-shrink-0 rounded object-cover" />`（16px、4px 圆角）。
     - `sourceAppIcon` 为空时不渲染 `<img>`，应用名文字保持现状（旧记录无图标）。
     - 类型彩色小标签（圆点 + 标签）完全不变。

### 图标缓存约定

- 缓存目录：`<app_data_dir>/app_icons/`
- 文件名：`<exe 路径的 hash>.png`（用 `DefaultHasher`，与现有 hash_value 风格一致）。
- 命中策略：拿到 exe 路径后先算 hash，检查缓存文件是否存在，存在则直接返回该路径，跳过提取。
- 提取失败（拿不到 HICON 或转 RGBA 失败）：`icon_path = None`，不写缓存文件，不报错。

### HICON → RGBA 转换要点

- `GetIconInfo(hicon)` → `ICONINFO { hbmColor, hbmMask }`。
- `GetDIBits` 对 color 位图取像素（`BITMAPINFO` 设 32bpp BI_RGB）。
- alpha 通道：若 color 位图本身带 alpha 用之；否则用 mask 位图——mask 中"透明"位置 alpha 置 0，否则 255。（Win32 图标 mask 是 1bpp，标准做法。）
- 完成后 `DeleteObject(hbmColor/hbmMask)`、`DestroyIcon(hicon)` 释放资源。

## 错误处理

- 整个 `get_source_app_info` 任何 Win32 调用失败都不 panic、不返回 Err（签名返回 `SourceAppInfo`，失败字段退化为空/None）：
  - 无前台窗口 → name="Unknown", icon=None
  - 拿不到 exe 路径 → name 用窗口标题 fallback, icon=None
  - `SHGFI_DISPLAYNAME` 失败 → name 用窗口标题 fallback
  - 图标提取/写文件失败 → icon=None
- 剪贴板捕获主流程因此绝不会被图标逻辑拖垮。

## 测试

- **单元测试**：
  - DB 迁移：在旧 schema（无 `source_app_icon` 列）的测试库上 `Database::new` 后，插入带 `source_app_icon` 的 entry 能正确读写。
  - `ClipboardEntry` 序列化含 `sourceAppIcon` 字段（serde camelCase）。
- **不可单测的部分**（依赖真实前台窗口/exe）：`get_source_app_info` 的 Win32 链路靠手动验证，不写单测。
- **手动验证清单**：
  1. 从 Chrome 复制文本 → 历史条目显示 Chrome 图标 + "Google Chrome"。
  2. 从不同应用复制 → 各自显示对应图标。
  3. 同一应用复制多次 → 只提取一次图标（缓存命中）。
  4. 旧数据库升级 → 应用不崩，旧记录无图标正常显示，新记录有图标。
  5. 某些 UWP/系统进程拿不到 exe → 不崩，退化为窗口标题或 "Unknown"，无图标。

## 影响范围

| 文件 | 改动 |
|------|------|
| `src-tauri/src/window_tracker.rs` | 新增 `SourceAppInfo` + `get_source_app_info`，Win32 图标提取 |
| `src-tauri/src/clipboard.rs` | 调用新函数；PNG 编码函数共享化 |
| `src-tauri/src/models.rs` | `ClipboardEntry` 加 `source_app_icon` |
| `src-tauri/src/db.rs` | schema 加列 + 迁移；SQL 补列 |
| `src-tauri/src/lib.rs` | 无需改命令（图标走现有 `read_image_file`） |
| `src-tauri/Cargo.toml` | `windows` features 需加 `Win32_Graphics_Gdi`（GetDIBits/ICONINFO）、`Win32_UI_Shell`（已有 SHGetFileInfo） |
| `src/types/index.ts` | `ClipboardEntry` 加 `sourceAppIcon` |
| `src/components/ClipboardItem.tsx` | 渲染图标 |
