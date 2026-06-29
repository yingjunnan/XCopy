# 剪贴板历史应用图标 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让每条剪贴板记录显示来源应用的真实图标，并把"应用名"从窗口标题改为进程产品名。

**Architecture:** 剪贴板捕获时通过 `GetForegroundWindow → PID → QueryFullProcessImageNameW` 拿到前台进程 exe 路径；用 `SHGetFileInfoW` 取产品名与 HICON；把 HICON 转 RGBA 再转 PNG，按 exe 路径 hash 缓存到 `app_icons/`；新列 `source_app_icon` 存缓存路径；前端用现有 `read_image_file` 命令取 base64 渲染 16px 图标。

**Tech Stack:** Rust + windows crate 0.58、SQLite (rusqlite)、React + TypeScript + Tailwind。

**Spec:** `docs/superpowers/specs/2026-06-29-clipboard-source-app-icon-design.md`

---

## File Structure

| 文件 | 责任 | 改动类型 |
|------|------|---------|
| `src-tauri/Cargo.toml` | 加 `Win32_Graphics_Gdi` feature | 修改 |
| `src-tauri/src/png_encode.rs` | RGBA→PNG 编码（从 clipboard.rs 抽出，共享） | 新建 |
| `src-tauri/src/window_tracker.rs` | `SourceAppInfo` + `get_source_app_info`，Win32 图标提取 | 修改 |
| `src-tauri/src/clipboard.rs` | 调用 `get_source_app_info`，去掉本地 PNG 函数 | 修改 |
| `src-tauri/src/models.rs` | `ClipboardEntry` 加 `source_app_icon` | 修改 |
| `src-tauri/src/db.rs` | schema 加列 + 迁移 + SQL 补列 | 修改 |
| `src-tauri/src/lib.rs` | `mod png_encode;` | 修改 |
| `src/types/index.ts` | `ClipboardEntry` 加 `sourceAppIcon` | 修改 |
| `src/components/ClipboardItem.tsx` | 渲染图标 | 修改 |

**关键约定（贯穿所有任务，保持一致）:**
- 新字段 Rust 名 `source_app_icon: Option<String>`，serde camelCase 序列化为 `sourceAppIcon`。
- `SourceAppInfo { name: String, icon_path: Option<PathBuf> }` 在 `window_tracker.rs`。
- 图标缓存目录名 `app_icons`，文件名 `<exe 路径 hash>.png`。
- PNG 编码函数提到 `png_encode.rs`：`pub fn rgba_to_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String>`（其余 helper 设为该模块私有）。

---

### Task 1: 抽出 PNG 编码模块

把 `clipboard.rs` 里的 `rgba_to_png / write_png_chunk / crc32 / deflate` 移到新模块 `png_encode.rs`，保持行为不变。先做这步，让后续 Task 4 的图标提取能复用。

**Files:**
- Create: `src-tauri/src/png_encode.rs`
- Modify: `src-tauri/src/clipboard.rs:268-373`（删掉 4 个函数体）
- Modify: `src-tauri/src/lib.rs:1`（加 `mod png_encode;`）

- [ ] **Step 1: 创建 png_encode.rs**

Create `src-tauri/src/png_encode.rs`:

```rust
/// Encode RGBA pixel data into a PNG.
///
/// `arboard` already normalizes the platform-native clipboard format
/// (BGRA on Windows) into standard RGBA before handing it to us, so the
/// bytes are used as-is without any channel swapping.
pub fn rgba_to_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    // The image is already RGBA; copy it straight through so channels aren't
    // swapped a second time (which is what produced the red/blue inversion).
    let rgba: Vec<u8> = bytes
        .chunks(4)
        .flat_map(|chunk| match chunk.len() {
            4 => chunk.to_vec(),
            3 => vec![chunk[0], chunk[1], chunk[2], 255],
            _ => chunk.to_vec(),
        })
        .collect();

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr_data.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr_data.push(8);
    ihdr_data.push(6);
    ihdr_data.push(0);
    ihdr_data.push(0);
    ihdr_data.push(0);
    write_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // IDAT
    let mut raw_data = Vec::with_capacity(height + rgba.len());
    for y in 0..height {
        raw_data.push(0);
        let start = y * width * 4;
        let end = start + width * 4;
        raw_data.extend_from_slice(&rgba[start..end.min(rgba.len())]);
    }

    let compressed = deflate(&raw_data);
    write_png_chunk(&mut png, b"IDAT", &compressed);
    write_png_chunk(&mut png, b"IEND", &[]);

    Ok(png)
}

fn write_png_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);
    let crc = crc32(chunk_type, data);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc_table = [0u32; 256];
    for (i, entry) in crc_table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xedb88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        *entry = c;
    }

    let mut crc: u32 = 0xffffffff;
    for &b in chunk_type.iter().chain(data.iter()) {
        let idx = ((crc as u8) ^ b) as usize;
        crc = crc_table[idx] ^ (crc >> 8);
    }
    !crc
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x78);
    output.push(0x01);

    let mut pos = 0;
    while pos < data.len() {
        let remaining = data.len() - pos;
        let block_size = remaining.min(65535);
        let is_final = pos + block_size >= data.len();

        output.push(if is_final { 0x01 } else { 0x00 });
        output.extend_from_slice(&(block_size as u16).to_le_bytes());
        output.extend_from_slice(&(!(block_size as u16)).to_le_bytes());
        output.extend_from_slice(&data[pos..pos + block_size]);
        pos += block_size;
    }

    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    output.extend_from_slice(&adler.to_be_bytes());

    output
}
```

- [ ] **Step 2: 在 lib.rs 注册模块**

Modify `src-tauri/src/lib.rs` line 1 — 在 `mod app_settings;` 之前加一行：

```rust
mod app_settings;
mod clipboard;
mod db;
mod hotkey_hook;
mod models;
mod png_encode;
mod quick_paste;
mod window_tracker;
```

- [ ] **Step 3: 从 clipboard.rs 删除已搬走的函数**

Modify `src-tauri/src/clipboard.rs` — 删除 `rgba_to_png / write_png_chunk / crc32 / deflate` 这 4 个函数（原 268-373 行，从 `/// Encode RGBA pixel data into a PNG.` 注释起到文件末尾 `deflate` 的闭合花括号）。

同时把对 `rgba_to_png` 的调用改为 `png_encode::rgba_to_png`。原 clipboard.rs 中：

```rust
let png_bytes = rgba_to_png(&img.bytes, img.width as usize, img.height as usize)?;
```

改为：

```rust
let png_bytes = png_encode::rgba_to_png(&img.bytes, img.width as usize, img.height as usize)?;
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过（warning 可有，error 无）。若报 `rgba_to_png not found`，确认调用处已改成 `png_encode::rgba_to_png`。

- [ ] **Step 5: 跑现有测试确认无回归**

Run: `cd src-tauri && cargo test`
Expected: 所有现有测试通过（clipboard / db 模块测试）。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/png_encode.rs src-tauri/src/clipboard.rs src-tauri/src/lib.rs
git commit -m "refactor: extract PNG encoder into png_encode module"
```

---

### Task 2: 数据模型 + DB schema 加列与迁移

为 `ClipboardEntry` 加 `source_app_icon` 字段，DB 加列并写迁移，所有 SQL 补列。这是数据层的地基，先 TDD。

**Files:**
- Modify: `src-tauri/src/models.rs:3-13`
- Modify: `src-tauri/src/db.rs`（建表、迁移、insert/query/get_last_entry）
- Test: `src-tauri/src/db.rs`（在现有 `#[cfg(test)] mod tests` 内追加）

- [ ] **Step 1: 给 ClipboardEntry 加字段**

Modify `src-tauri/src/models.rs` — 结构体加一个字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub content_type: String,
    pub content: String,
    pub source_app: String,
    pub source_app_icon: Option<String>,
    pub preview: String,
    pub created_at: String,
    pub image_path: Option<String>,
}
```

- [ ] **Step 2: 写失败测试 — 旧 schema 迁移后能读写新列**

在 `src-tauri/src/db.rs` 的 `#[cfg(test)] mod tests` 内追加（紧跟现有测试块，在 `}` 闭合前）：

```rust
    #[test]
    fn old_database_without_icon_column_migrates_and_stores_icon() {
        let dir = std::env::temp_dir().join(format!("xcopy-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // 模拟旧 schema：手动建一张没有 source_app_icon 列的表
        let db_path = dir.join("xcopy.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE clipboard_history (
                    id TEXT PRIMARY KEY,
                    content_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    source_app TEXT NOT NULL DEFAULT '',
                    preview TEXT NOT NULL DEFAULT '',
                    image_path TEXT,
                    created_at TEXT NOT NULL
                );
                PRAGMA journal_mode=WAL;",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO clipboard_history (id, content_type, content, source_app, preview, image_path, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params!["old-id", "text", "old content", "Old App", "old content", rusqlite::types::Null, (Utc::now() - ChronoDuration::days(1)).to_rfc3339()],
            )
            .unwrap();
        }

        // 现在用 Database::new 打开——应触发迁移加列
        let db = Database::new(dir).expect("migrated database should open");

        // 旧记录读出来：source_app_icon 应为 None
        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "old-id");
        assert_eq!(entries[0].source_app_icon, None);

        // 新插入带图标的记录：能读回
        let mut entry = text_entry("new content");
        entry.source_app_icon = Some("/some/path/chrome.png".to_string());
        assert!(db.insert_entry_if_changed(&entry).unwrap());

        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();
        assert_eq!(entries.len(), 2);
        let new_entry = entries.iter().find(|e| e.content == "new content").unwrap();
        assert_eq!(
            new_entry.source_app_icon,
            Some("/some/path/chrome.png".to_string())
        );
    }
```

注意：现有 `text_entry` helper（db.rs 测试模块内）构建 `ClipboardEntry` 时没设 `source_app_icon`，加了字段后会编译失败——下一步实现时统一补上。

- [ ] **Step 3: 运行测试确认失败**

Run: `cd src-tauri && cargo test -- old_database_without_icon_column_migrates`
Expected: 编译失败或测试失败（因为新字段未在 helpers 里初始化、迁移未实现、SQL 没带新列）。编译错误如 `missing field source_app_icon`。

- [ ] **Step 4: 修复测试 helpers**

在 `src-tauri/src/db.rs` 测试模块内，给 `text_entry`、`image_entry` helper 补上 `source_app_icon: None`：

```rust
    fn text_entry(content: &str) -> ClipboardEntry {
        ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: "text".to_string(),
            content: content.to_string(),
            source_app: "test".to_string(),
            source_app_icon: None,
            preview: content.to_string(),
            image_path: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }
```

```rust
    fn image_entry(path: PathBuf) -> ClipboardEntry {
        ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: "image".to_string(),
            content: "Image 1x1".to_string(),
            source_app: "test".to_string(),
            source_app_icon: None,
            preview: "1x1px image".to_string(),
            image_path: Some(path.to_string_lossy().to_string()),
            created_at: Utc::now().to_rfc3339(),
        }
    }
```

- [ ] **Step 5: 建表 SQL 加列**

Modify `src-tauri/src/db.rs` 的 `Database::new` 中 `CREATE TABLE` 语句，在 `source_app` 行后加 `source_app_icon`：

```rust
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                source_app TEXT NOT NULL DEFAULT '',
                source_app_icon TEXT,
                preview TEXT NOT NULL DEFAULT '',
                image_path TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_history(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_content_type ON clipboard_history(content_type);
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;",
        )
        .map_err(|e| e.to_string())?;
```

- [ ] **Step 6: 加迁移逻辑**

在 `src-tauri/src/db.rs` 的 `Database::new` 中，`execute_batch`（建表）之后、`Ok(Database { ... })` 之前，插入迁移：

```rust
        // Migration: older installs created the table without source_app_icon.
        // Add it if missing so existing users keep their history on upgrade.
        let needs_icon_column: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(clipboard_history)")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?;
            let mut found = false;
            for row in rows {
                if row.map_err(|e| e.to_string())? == "source_app_icon" {
                    found = true;
                    break;
                }
            }
            !found
        };
        if needs_icon_column {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN source_app_icon TEXT",
                [],
            )
            .map_err(|e| e.to_string())?;
        }
```

- [ ] **Step 7: 更新 insert_entry SQL**

Modify `src-tauri/src/db.rs` 的 `insert_entry` — SQL 加列与绑定：

```rust
    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<(), String> {
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT OR REPLACE INTO clipboard_history (id, content_type, content, source_app, source_app_icon, preview, image_path, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![entry.id, entry.content_type, entry.content, entry.source_app, entry.source_app_icon, entry.preview, entry.image_path, entry.created_at],
            ).map_err(|e| e.to_string())?;
        }
        self.prune_history()
    }
```

- [ ] **Step 8: 更新 query_entries SQL 与映射**

Modify `src-tauri/src/db.rs` 的 `query_entries` — SELECT 加列，绑定位次整体后移一位，映射加字段。

SELECT 语句改为：

```rust
        let mut sql = String::from(
            "SELECT id, content_type, content, source_app, source_app_icon, preview, image_path, created_at FROM clipboard_history WHERE 1=1"
        );
```

`query_map` 闭包改为（注意索引后移）：

```rust
        let entries = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    source_app: row.get(3)?,
                    source_app_icon: row.get(4)?,
                    preview: row.get(5)?,
                    image_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
```

- [ ] **Step 9: 更新 get_last_entry SQL 与映射**

Modify `src-tauri/src/db.rs` 的 `get_last_entry` — SELECT 与映射：

```rust
        let mut stmt = conn
            .prepare(
                "SELECT id, content_type, content, source_app, source_app_icon, preview, image_path, created_at
             FROM clipboard_history ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| e.to_string())?;

        let mut entries = stmt
            .query_map([], |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    source_app: row.get(3)?,
                    source_app_icon: row.get(4)?,
                    preview: row.get(5)?,
                    image_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
```

- [ ] **Step 10: 运行测试确认通过**

Run: `cd src-tauri && cargo test`
Expected: 全部通过，含新测试 `old_database_without_icon_column_migrates_and_stores_icon`。

- [ ] **Step 11: 提交**

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs
git commit -m "feat(db): add source_app_icon column with migration"
```

---

### Task 3: 后端 — 来源应用信息与图标提取

在 `window_tracker.rs` 实现 `SourceAppInfo` 与 `get_source_app_info`：拿 exe 路径、产品名、提取图标转 PNG 并缓存。这一步依赖 Task 1 的 `png_encode` 与 Task 2 的字段已落地。

**Files:**
- Modify: `src-tauri/Cargo.toml:27-38`（windows features 加 Gdi）
- Modify: `src-tauri/src/window_tracker.rs`（整文件重写 win 模块）

- [ ] **Step 1: Cargo.toml 加 Gdi feature**

Modify `src-tauri/Cargo.toml` — 在 windows features 列表加 `"Win32_Graphics_Gdi"`：

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Foundation",
    "Win32_System_DataExchange",
    "Win32_System_Ole",
    "Win32_System_Registry",
    "Win32_UI_Shell",
    "Win32_UI_TextServices",
    "Win32_Graphics_Gdi",
    "Win32_Storage_FileSystem",
] }
```

- [ ] **Step 2: 重写 window_tracker.rs win 模块**

Replace 整个 `src-tauri/src/window_tracker.rs` 内容为：

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::png_encode::rgba_to_png;

/// Source application info captured alongside a clipboard entry.
/// `icon_path` points at a cached PNG under `app_icons/`, or None if the
/// icon could not be extracted.
#[derive(Debug, Clone)]
pub struct SourceAppInfo {
    pub name: String,
    pub icon_path: Option<PathBuf>,
}

#[cfg(target_os = "windows")]
pub mod win {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
        HBITMAP,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetForegroundWindow, GetIconInfo, GetWindowThreadProcessId,
        GetWindowTextW, GetWindowTextLengthW, ICONINFO,
    };

    pub fn get_active_window_title() -> String {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0.is_null() {
                return String::from("Unknown");
            }
            let len = GetWindowTextLengthW(hwnd) as usize;
            if len == 0 {
                return String::from("Desktop");
            }
            let mut buf: Vec<u16> = vec![0; len + 1];
            let actual_len = GetWindowTextW(hwnd, &mut buf) as usize;
            if actual_len == 0 {
                return String::from("Desktop");
            }
            buf.truncate(actual_len);
            OsString::from_wide(&buf)
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Capture the foreground window's process exe path, product name, and icon.
    ///
    /// Every Win32 call here is best-effort: any failure degrades gracefully
    /// (name falls back to the window title, icon becomes None) so the
    /// clipboard capture flow is never blocked by icon extraction.
    pub fn get_source_app_info(app_data_dir: &Path) -> SourceAppInfo {
        let exe_path = match foreground_exe_path() {
            Some(p) => p,
            None => {
                return SourceAppInfo {
                    name: get_active_window_title(),
                    icon_path: None,
                };
            }
        };

        let name = display_name(&exe_path).unwrap_or_else(get_active_window_title);
        let icon_path = cached_icon_path(&exe_path, app_data_dir);

        SourceAppInfo { name, icon_path }
    }

    /// Get the full path of the exe that owns the foreground window.
    fn foreground_exe_path() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return None;
            }
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let exe = query_process_image_name(handle)?;
            let _ = CloseHandle(handle);
            Some(exe)
        }
    }

    fn query_process_image_name(handle: windows::Win32::Foundation::HANDLE) -> Option<String> {
        use windows::Win32::System::Threading::{
            QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        };
        unsafe {
            let mut buf = vec![0u16; 1024];
            let mut size = buf.len() as u32;
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut size)
                .ok()?;
            buf.truncate(size as usize);
            Some(OsString::from_wide(&buf).to_string_lossy().into_owned())
        }
    }

    /// Product/display name for an exe, via SHGetFileInfoW (e.g. "Google Chrome").
    fn display_name(exe_path: &str) -> Option<String> {
        // encode_utf16 for a proper null-terminated wide string (handles
        // non-ASCII paths like Chinese app install dirs correctly).
        let wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut shfi: SHFILEINFOW = Default::default();
        unsafe {
            let res = SHGetFileInfoW(
                windows::core::PCWSTR(wide.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut shfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_DISPLAYNAME,
            );
            if res == 0 {
                return None;
            }
        }
        let end = shfi.szDisplayName.iter().position(|&c| c == 0).unwrap_or(shfi.szDisplayName.len());
        if end == 0 {
            return None;
        }
        Some(OsString::from_wide(&shfi.szDisplayName[..end]).to_string_lossy().into_owned())
    }

    /// Return the cached icon PNG path for `exe_path`, extracting it first if missing.
    fn cached_icon_path(exe_path: &str, app_data_dir: &Path) -> Option<PathBuf> {
        let cache_file = icon_cache_file(exe_path, app_data_dir);
        if cache_file.exists() {
            return Some(cache_file);
        }

        let png = extract_icon_png(exe_path)?;
        let dir = app_data_dir.join("app_icons");
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        if std::fs::write(&cache_file, &png).is_err() {
            return None;
        }
        Some(cache_file)
    }

    fn icon_cache_file(exe_path: &str, app_data_dir: &Path) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        exe_path.hash(&mut hasher);
        let hash = hasher.finish();
        app_data_dir.join("app_icons").join(format!("{}.png", hash))
    }

    /// Extract the large icon for `exe_path` and encode it as PNG (RGBA).
    fn extract_icon_png(exe_path: &str) -> Option<Vec<u8>> {
        let wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut shfi: SHFILEINFOW = Default::default();
        unsafe {
            let res = SHGetFileInfoW(
                windows::core::PCWSTR(wide.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut shfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if res == 0 || shfi.hIcon.is_invalid() {
                return None;
            }
            let png = icon_to_png(shfi.hIcon);
            let _ = DestroyIcon(shfi.hIcon);
            png
        }
    }

    /// Convert an HICON to PNG bytes via GetIconInfo + GetDIBits.
    fn icon_to_png(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
        unsafe {
            let mut info: ICONINFO = Default::default();
            if GetIconInfo(hicon, &mut info).is_err() {
                return None;
            }

            // color bitmap gives us the RGBA pixels (sans alpha on many icons).
            let color = info.hbmColor;
            let mask = info.hbmMask;
            let result = (|| {
                if color.is_invalid() {
                    return None;
                }
                let (rgba, width, height) = bitmap_to_rgba(color)?;
                let alpha = mask_alpha(mask, width, height);
                let rgba = apply_alpha(rgba, alpha);
                rgba_to_png(&rgba, width, height).ok()
            })();

            if !color.is_invalid() {
                let _ = DeleteObject(color);
            }
            if !mask.is_invalid() {
                let _ = DeleteObject(mask);
            }
            result
        }
    }

    /// Read a 32bpp color bitmap into RGBA bytes plus dimensions.
    fn bitmap_to_rgba(bmp: HBITMAP) -> Option<(Vec<u8>, usize, usize)> {
        unsafe {
            let hdc = CreateCompatibleDC(None);
            if hdc.is_invalid() {
                return None;
            }
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = 0; // BI_RGB

            // First call with null buffer to query dimensions.
            let copied = GetDIBits(
                hdc,
                bmp,
                0,
                0,
                None,
                &mut bi,
                DIB_RGB_COLORS,
            );
            if copied == 0 {
                let _ = DeleteDC(hdc);
                return None;
            }

            let width = bi.bmiHeader.biWidth as usize;
            // biHeight is negative for top-down DIBs (what we want); abs it.
            let height = (bi.bmiHeader.biHeight.unsigned_abs()) as usize;
            if width == 0 || height == 0 {
                let _ = DeleteDC(hdc);
                return None;
            }

            let mut pixels = vec![0u8; width * height * 4];
            let copied = GetDIBits(
                hdc,
                bmp,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut core::ffi::c_void),
                &mut bi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(hdc);
            if copied == 0 {
                return None;
            }
            // GetDIBits gives BGRA on Windows; swap to RGBA for the PNG encoder.
            for chunk in pixels.chunks_mut(4) {
                chunk.swap(0, 2);
            }
            Some((pixels, width, height))
        }
    }

    /// Read the 1bpp mask bitmap; returns a Vec<u8> of 0/255 alpha (255 = opaque).
    /// Returns None if the mask is unusable; caller treats that as "all opaque".
    fn mask_alpha(mask: HBITMAP, width: usize, height: usize) -> Option<Vec<u8>> {
        unsafe {
            let hdc = CreateCompatibleDC(None);
            if hdc.is_invalid() {
                return None;
            }
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 1;
            bi.bmiHeader.biCompression = 0;
            bi.bmiHeader.biWidth = width as i32;
            bi.bmiHeader.biHeight = -(height as i32); // top-down
            bi.bmiHeader.biSizeImage = ((width + 7) / 8 * height) as u32;

            let mut buf = vec![0u8; bi.bmiHeader.biSizeImage as usize];
            let copied = GetDIBits(
                hdc,
                mask,
                0,
                height as u32,
                Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
                &mut bi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(hdc);
            if copied == 0 {
                return None;
            }

            // 1bpp: each bit, 0 = opaque, 1 = transparent. Rows padded to 4 bytes.
            let row_bytes = ((width + 31) / 32) * 4;
            let mut alpha = vec![255u8; width * height];
            for y in 0..height {
                for x in 0..width {
                    let byte = buf[y * row_bytes + x / 8];
                    if byte & (0x80 >> (x % 8)) != 0 {
                        alpha[y * width + x] = 0;
                    }
                }
            }
            Some(alpha)
        }
    }

    /// Overlay mask alpha onto the color RGBA. If color already has real alpha
    /// (non-zero), keep it; otherwise use the mask.
    fn apply_alpha(mut rgba: Vec<u8>, alpha: Vec<u8>) -> Vec<u8> {
        if alpha.len() == rgba.len() / 4 {
            for (i, &a) in alpha.iter().enumerate() {
                let p = i * 4;
                if rgba[p + 3] == 0 {
                    rgba[p + 3] = a;
                }
            }
        }
        rgba
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    use super::*;

    pub fn get_active_window_title() -> String {
        String::from("Unknown")
    }

    pub fn get_source_app_info(_app_data_dir: &Path) -> SourceAppInfo {
        SourceAppInfo {
            name: String::from("Unknown"),
            icon_path: None,
        }
    }
}
```

- [ ] **Step 3: 验证编译（cargo check）**

Run: `cd src-tauri && cargo check`
Expected: 编译通过。

若报 `biHeight` 类型错误（windows 0.58 中 `biHeight` 是 `i32`，`unsigned_abs()` 直接可用）。若 `FILE_FLAGS_AND_ATTRIBUTES` 找不到，确认 import 路径 `windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES`，且该类型由 `Win32_UI_Shell` 间接带出（已启用）——若报缺失，改用全限定名传 `0u32`-as-needed 或加 `Win32_Storage_FileSystem` feature（见下「若编译失败」排查）。

若编译报 `FILE_FLAGS_AND_ATTRIBUTES` 未找到：在 Cargo.toml windows features 追加 `"Win32_Storage_FileSystem"`，并把调用处 `windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0)` 保持不变。

- [ ] **Step 4: 跑测试确认无回归**

Run: `cd src-tauri && cargo test`
Expected: 全部通过。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/src/window_tracker.rs
git commit -m "feat: extract source app exe path, product name, and icon"
```

---

### Task 4: clipboard.rs 接入 get_source_app_info

把 `capture_from_clipboard` 里的 `let source_app = win::get_active_window_title();` 换成调用 `get_source_app_info`，并把 name 与 icon_path 填进两处 `ClipboardEntry` 构造。

**Files:**
- Modify: `src-tauri/src/clipboard.rs:173-245`（capture_from_clipboard）
- Modify: `src-tauri/src/clipboard.rs:156-163`（capture_current_clipboard 也需传 app_data_dir，已传入）

- [ ] **Step 1: 替换 source_app 获取与构造**

Modify `src-tauri/src/clipboard.rs` 的 `capture_from_clipboard` 开头。把：

```rust
    let source_app = win::get_active_window_title();
    let now = Utc::now().to_rfc3339();
```

改为：

```rust
    let source = win::get_source_app_info(app_data_dir);
    let now = Utc::now().to_rfc3339();
```

图片分支构造 entry 处，把 `source_app,` 改为 `source_app: source.name.clone(), source_app_icon: source.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),`：

```rust
        let entry = ClipboardEntry {
            id,
            content_type: "image".to_string(),
            content: format!("Image {}x{}", img.width, img.height),
            source_app: source.name.clone(),
            source_app_icon: source.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            preview: format!("{}x{}px image", img.width, img.height),
            created_at: now,
            image_path: Some(filepath.to_string_lossy().to_string()),
        };
```

文本分支构造 entry 处同样改：

```rust
        let entry = ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: detect_content_type(&text).to_string(),
            content: text.clone(),
            source_app: source.name.clone(),
            source_app_icon: source.icon_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            preview: truncate_preview(&text, 100),
            created_at: now,
            image_path: None,
        };
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check`
Expected: 通过。若 clipboard.rs 测试模块里有手动构造 `ClipboardEntry` 的地方（当前没有，都是 helper 函数），同样补 `source_app_icon: None`。

- [ ] **Step 3: 跑测试**

Run: `cd src-tauri && cargo test`
Expected: 全部通过。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/clipboard.rs
git commit -m "feat(clipboard): record source app name and icon on capture"
```

---

### Task 5: 前端类型与图标渲染

前端类型加 `sourceAppIcon`，`ClipboardItem.tsx` 渲染 16px / 4px 圆角图标。

**Files:**
- Modify: `src/types/index.ts:1-9`
- Modify: `src/components/ClipboardItem.tsx`

- [ ] **Step 1: 类型加字段**

Modify `src/types/index.ts`：

```typescript
export interface ClipboardEntry {
  id: string;
  contentType: 'text' | 'link' | 'image';
  content: string;
  sourceApp: string;
  sourceAppIcon?: string | null;
  preview: string;
  createdAt: string;
  imagePath?: string;
}
```

- [ ] **Step 2: ClipboardItem 加图标 state 与加载**

Modify `src/components/ClipboardItem.tsx` — 在现有 `imageSrc` state 旁加 `iconSrc`，并加 useEffect 加载。

把：

```tsx
  const [imageSrc, setImageSrc] = useState<string | null>(null);

  useEffect(() => {
    if (entry.contentType === "image" && entry.imagePath) {
      invoke<string>("read_image_file", { path: entry.imagePath })
        .then((data) => setImageSrc(`data:image/png;base64,${data}`))
        .catch(() => setImageSrc(null));
    }
  }, [entry.contentType, entry.imagePath]);
```

改为：

```tsx
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [iconSrc, setIconSrc] = useState<string | null>(null);

  useEffect(() => {
    if (entry.contentType === "image" && entry.imagePath) {
      invoke<string>("read_image_file", { path: entry.imagePath })
        .then((data) => setImageSrc(`data:image/png;base64,${data}`))
        .catch(() => setImageSrc(null));
    }
  }, [entry.contentType, entry.imagePath]);

  useEffect(() => {
    if (!entry.sourceAppIcon) {
      setIconSrc(null);
      return;
    }
    invoke<string>("read_image_file", { path: entry.sourceAppIcon })
      .then((data) => setIconSrc(`data:image/png;base64,${data}`))
      .catch(() => setIconSrc(null));
  }, [entry.sourceAppIcon]);
```

- [ ] **Step 3: 渲染图标 — 应用名左侧**

Modify `src/components/ClipboardItem.tsx` 的 meta 行。把原：

```tsx
          <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-slate-500">
            {entry.sourceApp || "未知应用"}
          </span>
```

改为（在应用名文字前插图标，仅在 iconSrc 存在时渲染）：

```tsx
          <span className="min-w-0 flex-1 flex items-center gap-1.5">
            {iconSrc && (
              <img
                src={iconSrc}
                alt=""
                className="h-4 w-4 flex-shrink-0 rounded-[4px] object-cover"
              />
            )}
            <span className="truncate font-mono text-[11px] text-slate-500">
              {entry.sourceApp || "未知应用"}
            </span>
          </span>
```

- [ ] **Step 4: 类型检查 + 构建**

Run: `npm run build`
Expected: TypeScript 编译通过，Vite 构建成功。

- [ ] **Step 5: 提交**

```bash
git add src/types/index.ts src/components/ClipboardItem.tsx
git commit -m "feat(ui): render source app icon in clipboard items"
```

---

### Task 6: 端到端手动验证

编译并启动应用，按手动清单验证图标与产品名正确显示、缓存命中、旧库升级不崩。

**Files:** 无（验证步骤）

- [ ] **Step 1: 构建应用**

Run: `npm run tauri dev`
Expected: 应用窗口启动，无 Rust/前端报错。

- [ ] **Step 2: 验证图标与产品名**

在浏览器(Chrome/Edge)里复制一段文本 → 按 `Ctrl+Shift+V` 唤起 → 确认该条目显示对应浏览器图标 + 产品名("Google Chrome"/"Microsoft Edge")，应用名不再是窗口标题。

- [ ] **Step 3: 验证缓存命中**

从同一应用再复制另一段文本 → 唤起确认有图标。检查 app_data_dir/app_icons/ 下只有一个对应 hash 的 png（同一应用只提取一次）。

可在 dev 控制台执行 `await import('@tauri-apps/api/path').then(p=>p.appDataDir())` 拿到路径查看 `app_icons/`。

- [ ] **Step 4: 验证旧库升级**

若已有旧 `xcopy.db`（无 source_app_icon 列）：应用启动后正常加载历史，旧记录无图标但正常显示，新复制的内容有图标。若没有旧库，可跳过——Task 2 的迁移测试已覆盖此场景。

- [ ] **Step 5: 验证降级兜底**

从某个拿不到 exe 的场景（如直接在桌面右键复制、或 UWP 设置界面）复制 → 确认应用不崩，退化为窗口标题或 "Unknown"，无图标，应用名文字正常。

- [ ] **Step 6: 若全部通过，无需提交（本任务无代码改动）**

若发现 bug，回到对应 Task 修复后再验证。
