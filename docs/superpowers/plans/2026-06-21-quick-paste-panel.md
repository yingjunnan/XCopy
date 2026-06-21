# 轻量快速粘贴面板(Quick Paste Panel)实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 双击 Ctrl 唤起跟随鼠标的轻量面板,选一条文本/链接历史后直接粘贴到当前光标位置。

**Architecture:** 新增低级键盘钩子(WH_KEYBOARD_LL)在后台线程检测双击 Ctrl → 记录目标窗口 HWND + 鼠标坐标 → 显示独立的 `quick-paste` Tauri 窗口 → 前端渲染最近文本/链接 → 用户选中 → Rust 写剪贴板 + SetForegroundWindow + SendInput(Ctrl+V) 注入。

**Tech Stack:** Rust + Tauri 2,windows crate 0.58(Win32 API),React + TypeScript + Tailwind,arboard(剪贴板)。

**关键约束(已核实):**
- 项目 Cargo.toml 已有的 windows feature 覆盖全部所需 API:`Win32_UI_WindowsAndMessaging`(SetWindowsHookExW/UnhookWindowsHookEx/CallNextHookEx/WH_KEYBOARD_LL/KBDLLHOOKSTRUCT/GetCursorPos/GetForegroundWindow/SetForegroundWindow)、`Win32_UI_Input_KeyboardAndMouse`(SendInput/INPUT_KEYBOARD/KEYBDINPUT/VK_CONTROL/VK_V/KEYEVENTF_KEYUP)、`Win32_System_Threading`(AttachThreadInput/GetCurrentThreadId)。**无需改 Cargo.toml 的 windows feature。**
- `ClipboardFilter.content_type` 是单值 `= ?` 匹配,无法一次取 text+link。前端取全部后在客户端过滤掉 image。
- standalone 窗口参考 `preview.html`/`preview.tsx` 模式,需在 `vite.config.ts` 的 `build.rollupOptions.input` 加入口。

---

## 文件结构

**新增:**
- `src-tauri/src/hotkey_hook.rs` — 低级键盘钩子 + 双击 Ctrl 检测状态机(纯逻辑可单测 + 安装/卸载)
- `src-tauri/src/quick_paste.rs` — 面板唤起(定位+显示)+ 粘贴注入(写剪贴板+激活+Ctrl+V)
- `src/quick_paste.tsx` — 面板前端入口
- `quick_paste.html` — 面板 HTML

**修改:**
- `src-tauri/src/app_settings.rs` — 新增 `quick_paste_enabled`、`double_click_interval_ms` 字段
- `src-tauri/src/lib.rs` — 注册命令、setup 安装钩子、窗口事件
- `src-tauri/tauri.conf.json` — 新增 quick-paste 窗口声明
- `src-tauri/capabilities/default.json` — 新窗口 + 权限
- `src/types/index.ts` — AppSettings 新字段
- `src/components/SettingsPanel.tsx` — 双击唤起开关 UI
- `vite.config.ts` — 新入口

---

## Task 1: AppSettings 新增双击唤起字段(TDD)

**Files:**
- Modify: `src-tauri/src/app_settings.rs`

- [ ] **Step 1: 写失败测试 — 默认值**

在 `app_settings.rs` 的 `mod tests` 末尾追加:

```rust
    #[test]
    fn default_settings_enable_quick_paste_with_300ms_interval() {
        let settings = AppSettings::default();

        assert!(settings.quick_paste_enabled);
        assert_eq!(settings.double_click_interval_ms, 300);
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib default_settings_enable_quick_paste_with_300ms_interval`
Expected: 编译错误,`quick_paste_enabled` 字段不存在。

- [ ] **Step 3: 实现 — 加字段、默认值、normalize**

在 `AppSettings` struct 加两个字段:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_start: bool,
    pub shortcut: String,
    #[serde(default = "default_max_history_entries")]
    pub max_history_entries: usize,
    #[serde(default = "default_retention_days")]
    pub retention_days: i64,
    #[serde(default = "default_quick_paste_enabled")]
    pub quick_paste_enabled: bool,
    #[serde(default = "default_double_click_interval_ms")]
    pub double_click_interval_ms: u32,
}
```

加默认值函数(放在 `default_retention_days` 函数后):

```rust
fn default_quick_paste_enabled() -> bool {
    true
}

fn default_double_click_interval_ms() -> u32 {
    300
}
```

更新 `Default` impl:

```rust
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
            max_history_entries: DEFAULT_MAX_HISTORY_ENTRIES,
            retention_days: DEFAULT_RETENTION_DAYS,
            quick_paste_enabled: true,
            double_click_interval_ms: 300,
        }
    }
}
```

加常量(放在其他常量旁):

```rust
pub const MIN_DOUBLE_CLICK_INTERVAL_MS: u32 = 200;
pub const MAX_DOUBLE_CLICK_INTERVAL_MS: u32 = 400;
```

更新 `normalize_settings`(在 `settings.retention_days.clamp(...)` 后加):

```rust
    settings.double_click_interval_ms = settings
        .double_click_interval_ms
        .clamp(MIN_DOUBLE_CLICK_INTERVAL_MS, MAX_DOUBLE_CLICK_INTERVAL_MS);
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib app_settings`
Expected: 8 passed; 0 failed。

- [ ] **Step 5: 补 roundtrip/legacy 测试**

更新现有的 `saves_and_loads_settings_roundtrip` 测试里的 settings 构造,补上新字段:

```rust
    #[test]
    fn saves_and_loads_settings_roundtrip() {
        let path = temp_settings_path("roundtrip");
        let settings = AppSettings {
            auto_start: true,
            shortcut: "control+alt+KeyX".to_string(),
            max_history_entries: 250,
            retention_days: 14,
            quick_paste_enabled: false,
            double_click_interval_ms: 250,
        };
```

(断言里的构造体也加 `quick_paste_enabled: false, double_click_interval_ms: 250,`。)

加 legacy 测试(老 settings 文件无新字段时用默认):

```rust
    #[test]
    fn loads_older_settings_with_default_quick_paste_values() {
        let path = temp_settings_path("legacy-quickpaste");
        std::fs::write(
            &path,
            r#"{"autoStart":true,"shortcut":"Ctrl+Shift+V","maxHistoryEntries":100,"retentionDays":7}"#,
        )
        .unwrap();

        let loaded = load_settings_from_path(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(loaded.quick_paste_enabled);
        assert_eq!(loaded.double_click_interval_ms, 300);
    }
```

- [ ] **Step 6: 运行全部测试**

Run: `cargo test --lib app_settings`
Expected: 全部通过(含 roundtrip、legacy-quickpaste、clamp)。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/app_settings.rs
git commit -m "feat(settings): add quick_paste_enabled and double_click_interval_ms fields"
```

---

## Task 2: 双击 Ctrl 检测状态机(TDD 纯逻辑)

**Files:**
- Create: `src-tauri/src/hotkey_hook.rs`

- [ ] **Step 1: 写失败测试 — 状态机判定**

创建 `hotkey_hook.rs`,先写纯逻辑 + 测试:

```rust
use std::time::Instant;

/// 纯逻辑双击检测器:不接触 Win32,便于单测。
/// 判定规则:两次 Ctrl keydown 间隔在 [min_ms, max_ms] 内算命中。
pub struct DoubleClickDetector {
    last_press: Option<Instant>,
    min_interval_ms: u32,
    max_interval_ms: u32,
}

impl DoubleClickDetector {
    pub fn new(min_interval_ms: u32, max_interval_ms: u32) -> Self {
        Self {
            last_press: None,
            min_interval_ms,
            max_interval_ms,
        }
    }

    /// 记录一次 Ctrl keydown,返回是否构成双击(命中后重置)。
    pub fn on_ctrl_press(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last_press {
            let elapsed = now.duration_since(last).as_millis() as u32;
            if elapsed >= self.min_interval_ms && elapsed <= self.max_interval_ms {
                self.last_press = None;
                return true;
            }
        }
        self.last_press = Some(now);
        false
    }

    /// Ctrl keyup 不影响判定(只看 keydown),但用于将来扩展。
    pub fn on_ctrl_release(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(millis: u64) -> Instant {
        Instant::now() + Duration::from_millis(millis) - Duration::from_millis(millis)
    }

    #[test]
    fn single_press_does_not_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        assert!(!d.on_ctrl_press(Instant::now()));
    }

    #[test]
    fn two_presses_within_window_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        assert!(!d.on_ctrl_press(t0));
        let t1 = t0 + Duration::from_millis(300);
        assert!(d.on_ctrl_press(t1));
    }

    #[test]
    fn two_presses_too_close_do_not_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        assert!(!d.on_ctrl_press(t0));
        let t1 = t0 + Duration::from_millis(100);
        assert!(!d.on_ctrl_press(t1));
    }

    #[test]
    fn two_presses_too_far_do_not_trigger() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        assert!(!d.on_ctrl_press(t0));
        let t1 = t0 + Duration::from_millis(500);
        assert!(!d.on_ctrl_press(t1));
    }

    #[test]
    fn trigger_resets_so_third_press_starts_new_window() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        d.on_ctrl_press(t0);
        let t1 = t0 + Duration::from_millis(300);
        assert!(d.on_ctrl_press(t1)); // 命中并重置
        let t2 = t1 + Duration::from_millis(300);
        assert!(!d.on_ctrl_press(t2)); // 新窗口的第一次,不触发
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --lib hotkey_hook`
Expected: 编译失败,因为 `hotkey_hook` 未在 lib.rs 声明。

- [ ] **Step 3: 在 lib.rs 声明模块**

在 `lib.rs` 的 `mod` 声明区(已有 `mod app_settings;` 等)加:

```rust
mod hotkey_hook;
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib hotkey_hook`
Expected: 5 passed; 0 failed。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/hotkey_hook.rs src-tauri/src/lib.rs
git commit -m "feat(hotkey): add pure double-click Ctrl detector with unit tests"
```

---

## Task 3: 低级键盘钩子安装与回调

**Files:**
- Modify: `src-tauri/src/hotkey_hook.rs`(加 Win32 钩子部分)
- Modify: `src-tauri/src/lib.rs`(setup 阶段安装钩子)

- [ ] **Step 1: 在 hotkey_hook.rs 加 Windows 钩子实现**

在文件末尾(tests 模块之前)加:

```rust
#[cfg(target_os = "windows")]
pub mod win {
    use std::sync::Mutex;
    use std::time::Instant;
    use tauri::{AppHandle, Emitter, Manager};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WINDOWS_HOOK_ID,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_LCONTROL, VK_RCONTROL};

    const WM_KEYDOWN: u32 = 0x0100;
    const WM_SYSKEYDOWN: u32 = 0x0104;

    /// 全局状态:双击检测器 + 是否启用 + 唤起回调用的 AppHandle。
    /// 钩子回调在独立线程跑消息循环,通过 Mutex 访问。
    struct HookState {
        detector: DoubleClickDetector,
        enabled: bool,
        app_handle: AppHandle,
    }

    static HOOK_STATE: Mutex<Option<HookState>> = Mutex::new(None);
    static HOOK_HANDLE: Mutex<Option<HHOOK>> = Mutex::new(None);

    /// 钩子回调。只检测 Ctrl keydown 做双击判定,不消费任何按键。
    unsafe extern "system" fn hook_proc(
        _code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let wparam_u = wparam.0 as u32;
        if wparam_u == WM_KEYDOWN || wparam_u == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode;
            if vk == VK_CONTROL.0 || vk == VK_LCONTROL.0 || vk == VK_RCONTROL.0 {
                let now = Instant::now();
                let mut state_guard = HOOK_STATE.lock().unwrap();
                if let Some(state) = state_guard.as_mut() {
                    if state.enabled && state.detector.on_ctrl_press(now) {
                        let handle = state.app_handle.clone();
                        // 在锁外触发,避免死锁/阻塞钩子链。
                        drop(state_guard);
                        let _ = handle.emit("quick-paste-trigger", ());
                    }
                }
            }
        }
        CallNextHookEx(None, _code, wparam, lparam)
    }

    /// 安装低级键盘钩子并启动消息循环。在独立线程调用。
    /// 间隔 ms 从 settings 传入。
    pub fn install(app_handle: AppHandle, interval_ms: u32, enabled: bool) {
        {
            let mut state = HOOK_STATE.lock().unwrap();
            *state = Some(HookState {
                detector: DoubleClickDetector::new(200, 400),
                enabled,
                app_handle: app_handle.clone(),
            });
        }

        std::thread::spawn(move || {
            unsafe {
                let hook = SetWindowsHookExW(WINDOWS_HOOK_ID(WH_KEYBOARD_LL.0), Some(hook_proc), None, 0);
                if let Ok(hook) = hook {
                    let mut handle_guard = HOOK_HANDLE.lock().unwrap();
                    *handle_guard = Some(hook);

                    // 消息循环:低级钩子必须有消息泵,否则会被系统卸载。
                    let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                    while windows::Win32::UI::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).into() {
                        let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                        windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                    }

                    let _ = UnhookWindowsHookEx(hook);
                }
            }
        });
    }

    /// 动态开关:更新 enabled 状态(不改间隔,间隔需重装)。
    pub fn set_enabled(enabled: bool) {
        if let Some(state) = HOOK_STATE.lock().unwrap().as_mut() {
            state.enabled = enabled;
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    use tauri::AppHandle;
    pub fn install(_app_handle: AppHandle, _interval_ms: u32, _enabled: bool) {}
    pub fn set_enabled(_enabled: bool) {}
}
```

- [ ] **Step 2: 在 lib.rs setup 阶段安装钩子**

在 `lib.rs` 的 `run()` 函数的 `.setup(|app| { ... })` 闭包里,在 `register_app_shortcut(app.handle(), &settings.shortcut)?;` 之后、`setup_tray(app)?;` 之前加:

```rust
            hotkey_hook::win::install(
                app.handle().clone(),
                settings.double_click_interval_ms,
                settings.quick_paste_enabled,
            );
```

- [ ] **Step 3: 在 save_app_settings 里同步开关**

在 `lib.rs` 的 `save_app_settings` 命令里,`app_settings::set_auto_start_enabled(...)?;` 之后加:

```rust
    hotkey_hook::win::set_enabled(next_settings.quick_paste_enabled);
```

- [ ] **Step 4: 编译确认**

Run: `cargo build` (在 src-tauri 目录)
Expected: 编译通过。若 `GetMessageW`/`TranslateMessage`/`DispatchMessageW`/`MSG` 未在现有 feature 下,可能需确认 —— 这些都在 `Win32_UI_WindowsAndMessaging`,已有。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/hotkey_hook.rs src-tauri/src/lib.rs
git commit -m "feat(hotkey): install WH_KEYBOARD_LL hook to detect double-tap Ctrl"
```

---

## Task 4: quick_paste.rs — 粘贴注入逻辑

**Files:**
- Create: `src-tauri/src/quick_paste.rs`

- [ ] **Step 1: 创建 quick_paste.rs 粘贴注入实现**

```rust
#[cfg(target_os = "windows")]
pub mod win {
    use std::thread;
    use std::time::Duration;
    use tauri::{AppHandle, Manager};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};

    /// 粘贴前记录的目标窗口 HWND(唤起时由 show_quick_paste_panel 写入)。
    static TARGET_HWND: std::sync::Mutex<Option<isize>> = std::sync::Mutex::new(None);

    pub fn remember_target_window() {
        unsafe {
            let hwnd = GetForegroundWindow();
            *TARGET_HWND.lock().unwrap() = if hwnd.0.is_null() { None } else { Some(hwnd.0 as isize) };
        }
    }

    /// 写入剪贴板(文本)→ 激活目标窗口 → 发送 Ctrl+V。
    pub fn paste_text(content: &str) -> Result<(), String> {
        // 1. 写剪贴板
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(content.to_string()).map_err(|e| e.to_string())?;

        // 2. 激活目标窗口
        let target = TARGET_HWND.lock().unwrap().map(|v| HWND(v as *mut _));
        if let Some(hwnd) = target {
            bring_window_to_foreground(hwnd);
        }

        // 3. 等焦点稳定后发 Ctrl+V
        thread::sleep(Duration::from_millis(80));
        send_ctrl_v();
        Ok(())
    }

    /// 绕过 SetForegroundWindow 限制:AttachThreadInput 把目标线程附着到当前线程。
    fn bring_window_to_foreground(hwnd: HWND) {
        unsafe {
            let target_thread = GetWindowThreadProcessId(hwnd, None);
            let current_thread = GetCurrentThreadId();
            // 先发一次 Alt 键解锁 foreground 锁(常见绕过手法)。
            let _ = SetForegroundWindow(hwnd);
            if target_thread != current_thread {
                let _ = AttachThreadInput(current_thread, target_thread, true);
                let _ = SetForegroundWindow(hwnd);
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }
        }
    }

    /// 用 SendInput 发 Ctrl+V(扫描码方式)。
    fn send_ctrl_v() {
        unsafe {
            let mut inputs: [INPUT; 4] = std::mem::zeroed();

            // Ctrl down
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            // V down
            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: VK_V,
                wScan: 0,
                dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            // V up
            inputs[2].r#type = INPUT_KEYBOARD;
            inputs[2].Anonymous.ki = KEYBDINPUT {
                wVk: VK_V,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            // Ctrl up
            inputs[3].r#type = INPUT_KEYBOARD;
            inputs[3].Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };

            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    pub fn remember_target_window() {}
    pub fn paste_text(_content: &str) -> Result<(), String> {
        Err("不支持当前平台".to_string())
    }
}
```

- [ ] **Step 2: 在 lib.rs 声明模块**

`mod` 区加:

```rust
mod quick_paste;
```

- [ ] **Step 3: 编译确认**

Run: `cargo build`
Expected: 编译通过。注意 windows 0.58 里 `INPUT` 的 union 字段是 `Anonymous`,确认无误。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/quick_paste.rs src-tauri/src/lib.rs
git commit -m "feat(quick-paste): add clipboard write + Ctrl+V injection via SendInput"
```

---

## Task 5: show_quick_paste_panel + paste_from_quick_paste 命令

**Files:**
- Modify: `src-tauri/src/quick_paste.rs`(加 Tauri 命令)
- Modify: `src-tauri/src/lib.rs`(注册命令)
- Modify: `src-tauri/tauri.conf.json`(窗口声明)
- Modify: `src-tauri/capabilities/default.json`(权限)

- [ ] **Step 1: 在 quick_paste.rs 加窗口定位 + 显示逻辑**

在 `win` 模块加(Windows 部分):

```rust
    use tauri::{LogicalPosition, Manager};
    use windows::Win32::Foundation::POINT;

    const PANEL_WIDTH: i32 = 320;
    const PANEL_HEIGHT: i32 = 360;

    /// 计算面板位置:跟随鼠标但不出屏(clamp)。
    pub fn compute_panel_position(cursor_x: i32, cursor_y: i32, screen_w: i32, screen_h: i32) -> (i32, i32) {
        let mut x = cursor_x;
        let mut y = cursor_y;
        if x + PANEL_WIDTH > screen_w {
            x = screen_w - PANEL_WIDTH;
        }
        if y + PANEL_HEIGHT > screen_h {
            y = screen_h - PANEL_HEIGHT;
        }
        if x < 0 { x = 0; }
        if y < 0 { y = 0; }
        (x, y)
    }

    /// 唤起面板:记录目标窗口 + 取鼠标坐标 + 定位 + 显示。
    pub fn show_panel(app: &tauri::AppHandle) {
        remember_target_window();

        let (cursor_x, cursor_y) = get_cursor_pos();
        let (screen_w, screen_h) = get_screen_size();
        let (x, y) = compute_panel_position(cursor_x, cursor_y, screen_w, screen_h);

        if let Some(window) = app.get_webview_window("quick-paste") {
            let _ = window.set_position(LogicalPosition::new(x as f64, y as f64));
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn get_cursor_pos() -> (i32, i32) {
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point);
            (point.x, point.y)
        }
    }

    fn get_screen_size() -> (i32, i32) {
        unsafe {
            let cx = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
            );
            let cy = windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
            );
            (cx, cy)
        }
    }
```

非 Windows 桩:

```rust
    pub fn compute_panel_position(cursor_x: i32, cursor_y: i32, screen_w: i32, screen_h: i32) -> (i32, i32) {
        (cursor_x, cursor_y)
    }
    pub fn show_panel(_app: &tauri::AppHandle) {}
```

- [ ] **Step 2: 加纯逻辑测试 for compute_panel_position**

在 quick_paste.rs 加测试模块:

```rust
#[cfg(test)]
mod tests {
    use super::win::compute_panel_position;

    #[cfg(target_os = "windows")]
    #[test]
    fn panel_clamps_when_cursor_near_bottom_right() {
        // 鼠标在右下角,面板应向左上偏移以不出屏。
        let (x, y) = compute_panel_position(1900, 1050, 1920, 1080);
        assert!(x + 320 <= 1920);
        assert!(y + 360 <= 1080);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn panel_stays_when_cursor_centered() {
        let (x, y) = compute_panel_position(500, 400, 1920, 1080);
        assert_eq!(x, 500);
        assert_eq!(y, 400);
    }
}
```

- [ ] **Step 3: 在 lib.rs 加 Tauri 命令 + 触发监听**

在 lib.rs 命令区加:

```rust
#[tauri::command]
fn show_quick_paste_panel(app: tauri::AppHandle) -> Result<(), String> {
    quick_paste::win::show_panel(&app);
    Ok(())
}

#[tauri::command]
fn paste_from_quick_paste(app: tauri::AppHandle, content: String) -> Result<(), String> {
    quick_paste::win::paste_text(&content)?;
    if let Some(window) = app.get_webview_window("quick-paste") {
        let _ = window.hide();
    }
    Ok(())
}
```

注册到 invoke_handler(在现有 `generate_handler!` 数组里加两个):

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

在 setup 闭包里加触发事件监听(在 `hotkey_hook::win::install(...)` 之后):

```rust
            // 双击 Ctrl 触发:显示 quick-paste 面板。
            let trigger_handle = app.handle().clone();
            app.listen("quick-paste-trigger", move |_event| {
                quick_paste::win::show_panel(&trigger_handle);
            });
```

- [ ] **Step 4: tauri.conf.json 加 quick-paste 窗口声明**

在 `app.windows` 数组里(现有 main 窗口之后)加:

```json
      {
        "label": "quick-paste",
        "url": "quick_paste.html",
        "title": "Quick Paste",
        "width": 320,
        "height": 360,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "visible": false,
        "skipTaskbar": true,
        "center": false,
        "shadow": true
      }
```

- [ ] **Step 5: capabilities/default.json 加窗口 + 权限**

把 `windows` 数组改为:

```json
  "windows": ["main", "image-preview", "quick-paste"],
```

`permissions` 数组里加(若无):

```json
    "core:window:allow-set-position",
    "core:window:allow-hide",
```

(`allow-show`、`allow-set-focus` 已有。)

- [ ] **Step 6: 编译 + 测试**

Run: `cargo test --lib quick_paste && cargo build`
Expected: 测试通过,编译通过。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/quick_paste.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat(quick-paste): wire show/paste commands, window declaration, capabilities"
```

---

## Task 6: 前端 quick_paste.html + quick_paste.tsx

**Files:**
- Create: `quick_paste.html`
- Create: `src/quick_paste.tsx`
- Modify: `vite.config.ts`

- [ ] **Step 1: 创建 quick_paste.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Quick Paste - XCopy</title>
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
    <script type="module" src="/src/quick_paste.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: 创建 src/quick_paste.tsx**

```tsx
import React, { useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import type { ClipboardEntry, ClipboardFilter } from "./types";

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const QuickPastePanel: React.FC = () => {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(true);

  const loadHistory = useCallback(async () => {
    if (!isTauriRuntime()) {
      setLoading(false);
      return;
    }
    try {
      const filter: ClipboardFilter = { limit: 50, offset: 0 };
      const all = await invoke<ClipboardEntry[]>("get_history", { filter });
      // 排除图片:quick-paste 只处理文本/链接
      const filtered = all.filter((e) => e.contentType !== "image");
      setEntries(filtered);
      setSelected(0);
    } catch (err) {
      console.error("Failed to load history:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const paste = useCallback(
    async (entry: ClipboardEntry) => {
      if (!isTauriRuntime()) return;
      try {
        await invoke("paste_from_quick_paste", { content: entry.content });
      } catch (err) {
        console.error("Paste failed:", err);
      }
    },
    []
  );

  const close = useCallback(() => {
    if (isTauriRuntime()) {
      getCurrentWindow().hide().catch(() => {});
    }
  }, []);

  // 键盘导航:↑↓ 选择,Enter 粘贴,Esc 关闭
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((s) => Math.min(s + 1, entries.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const entry = entries[selected];
        if (entry) paste(entry);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [entries, selected, paste, close]);

  // 失焦即隐藏(无延迟,区别于主窗口)
  useEffect(() => {
    if (!isTauriRuntime()) return;
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) close();
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [close]);

  // 窗口每次显示时重新加载
  useEffect(() => {
    if (!isTauriRuntime()) return;
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) loadHistory();
    });
    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [loadHistory]);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-xl bg-white shadow-2xl">
      <div className="flex-1 overflow-y-auto py-2">
        {loading ? (
          <div className="px-4 py-8 text-center text-[12px] text-slate-400">
            加载中...
          </div>
        ) : entries.length === 0 ? (
          <div className="px-4 py-8 text-center text-[12px] text-slate-400">
            暂无文本记录
          </div>
        ) : (
          entries.map((entry, index) => (
            <div
              key={entry.id}
              onClick={() => paste(entry)}
              onMouseEnter={() => setSelected(index)}
              className={`
                cursor-pointer border-l-2 px-3 py-2 transition-colors
                ${index === selected
                  ? "border-[#0067c0] bg-[#0067c0]/8"
                  : "border-transparent hover:bg-slate-50"}
              `}
            >
              <div className="flex items-center gap-2">
                <span
                  className={`h-1.5 w-1.5 flex-shrink-0 rounded-full ${
                    entry.contentType === "link" ? "bg-[#107c10]" : "bg-[#0067c0]"
                  }`}
                />
                <span className="min-w-0 flex-1 truncate text-[12px] text-slate-800">
                  {entry.preview}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
      <div className="border-t border-slate-100 px-3 py-1.5 text-[10px] text-slate-400">
        ↑↓ 选择 · Enter 粘贴 · Esc 关闭
      </div>
    </div>
  );
};

createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QuickPastePanel />
  </React.StrictMode>
);
```

- [ ] **Step 3: vite.config.ts 加入口**

把 `build.rollupOptions.input` 改为:

```typescript
      input: {
        main: resolve(__dirname, "index.html"),
        preview: resolve(__dirname, "preview.html"),
        quickPaste: resolve(__dirname, "quick_paste.html"),
      },
```

- [ ] **Step 4: 前端构建确认**

Run: `npm run build`
Expected: 构建成功,dist 里有 quick_paste.html。

- [ ] **Step 5: 提交**

```bash
git add quick_paste.html src/quick_paste.tsx vite.config.ts
git commit -m "feat(quick-paste): add lightweight paste panel frontend with keyboard nav"
```

---

## Task 7: SettingsPanel 加双击唤起开关 UI

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/components/SettingsPanel.tsx`

- [ ] **Step 1: types/index.ts 扩展 AppSettings**

```typescript
export interface AppSettings {
  autoStart: boolean;
  shortcut: string;
  maxHistoryEntries: number;
  retentionDays: number;
  quickPasteEnabled: boolean;
  doubleClickIntervalMs: number;
}
```

- [ ] **Step 2: SettingsPanel.tsx — 默认值 + 开关**

把 `DEFAULT_SETTINGS` 改为:

```typescript
const DEFAULT_SETTINGS: AppSettings = {
  autoStart: false,
  shortcut: "Ctrl+Shift+V",
  maxHistoryEntries: 1000,
  retentionDays: 30,
  quickPasteEnabled: true,
  doubleClickIntervalMs: 300,
};
```

在 `changed` 的 useMemo 里加判断:

```typescript
  const changed = useMemo(
    () =>
      settings.autoStart !== savedSettings.autoStart ||
      settings.shortcut !== savedSettings.shortcut ||
      settings.maxHistoryEntries !== savedSettings.maxHistoryEntries ||
      settings.retentionDays !== savedSettings.retentionDays ||
      settings.quickPasteEnabled !== savedSettings.quickPasteEnabled,
    [settings, savedSettings]
  );
```

加 toggle(在 `toggleAutoStart` 后):

```typescript
  const toggleQuickPaste = useCallback(() => {
    setMessage("");
    setError("");
    setSettings((current) => ({
      ...current,
      quickPasteEnabled: !current.quickPasteEnabled,
    }));
  }, []);
```

- [ ] **Step 3: SettingsPanel.tsx — 加 UI section**

在"开机自启" section 之后插入新 section:

```tsx
          <section className="rounded-xl border border-slate-900/[0.08] bg-white p-4 shadow-[0_1px_2px_rgba(31,41,55,0.05)]">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <h2 className="text-[13px] font-semibold text-slate-900">双击 Ctrl 快速粘贴</h2>
                <p className="mt-1 text-[11px] leading-4 text-slate-500">
                  {settings.quickPasteEnabled ? "已开启" : "已关闭"} · 双击 Ctrl 在光标处弹出选择条
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.quickPasteEnabled}
                onClick={toggleQuickPaste}
                disabled={loading || saving}
                className={`
                  relative h-6 w-11 rounded-full transition-colors duration-200
                  ${settings.quickPasteEnabled ? "bg-[#0067c0]" : "bg-slate-300"}
                  disabled:cursor-not-allowed disabled:opacity-60
                `}
              >
                <span
                  className={`
                    absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm
                    transition-transform duration-200
                    ${settings.quickPasteEnabled ? "translate-x-5" : "translate-x-0"}
                  `}
                />
              </button>
            </div>
          </section>
```

- [ ] **Step 4: 构建确认**

Run: `npm run build`
Expected: 构建成功。

- [ ] **Step 5: 提交**

```bash
git add src/types/index.ts src/components/SettingsPanel.tsx
git commit -m "feat(settings): add quick paste toggle UI in settings panel"
```

---

## Task 8: 集成构建 + 手测验证

- [ ] **Step 1: 完整构建**

Run: `npm run tauri build`
Expected: 生成 `src-tauri/target/release/bundle/nsis/XCopy_0.1.0_x64-setup.exe`,无编译错误。

- [ ] **Step 2: 手测清单(逐项验证,记录结果)**

1. 安装后运行 → 在记事本里复制几段文本
2. 在记事本里双击 Ctrl → 面板应在鼠标位置弹出,显示最近文本
3. ↑↓ 键选中 → Enter → 内容应出现在记事本光标处
4. 点击某条 → 同样粘贴成功
5. Esc 关闭面板
6. 点击面板外 → 面板失焦自动隐藏
7. 鼠标在屏幕右下角双击 → 面板不超出屏幕
8. 设置页关闭"双击 Ctrl 快速粘贴" → 双击无反应;开启后恢复
9. 主窗口 Ctrl+Shift+V 与双击 Ctrl 面板互不干扰
10. 空历史时面板显示"暂无文本记录"

- [ ] **Step 3: 修复发现的问题(如有)**

针对手测发现的问题逐个修复。

- [ ] **Step 4: 最终提交**

```bash
git add -A
git commit -m "feat(quick-paste): complete lightweight paste panel with double-tap Ctrl"
```

---

## Self-Review 备注

- Spec 覆盖:双击检测(Task 2-3)、粘贴注入(Task 4)、窗口与命令(Task 5)、前端(Task 6)、设置开关(Task 1,7)、手测(Task 8)全部覆盖。
- 焦点时序、AttachThreadInput 绕过、clamp 纯函数测试、失焦隐藏、UIPI 限制(已在 spec 注明,不在计划修复)均处理。
- windows 0.58 feature 已核实无需改 Cargo.toml。
