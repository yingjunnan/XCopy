#[cfg(target_os = "windows")]
pub mod win {
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;
    use tauri::Manager;
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
        VK_CONTROL, VK_V,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };

    /// 粘贴前记录的目标窗口 HWND(唤起时由 show_panel 写入)。
    /// 存 isize 而非 HWND,因为 HWND(*mut c_void) 不是 Send,不能放 static。
    static TARGET_HWND: Mutex<Option<isize>> = Mutex::new(None);

    /// 唤起时调用:把当前前台窗口记为粘贴目标。
    pub fn remember_target_window() {
        unsafe {
            let hwnd = GetForegroundWindow();
            *TARGET_HWND.lock().unwrap() =
                if hwnd.0.is_null() { None } else { Some(hwnd.0 as isize) };
        }
    }

    /// 写入剪贴板(文本)→ 激活目标窗口 → 发送 Ctrl+V。
    pub fn paste_text(content: &str) -> Result<(), String> {
        // 1. 写剪贴板
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard
            .set_text(content.to_string())
            .map_err(|e| e.to_string())?;

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
            let _ = SetForegroundWindow(hwnd);
            if target_thread != current_thread {
                let _ = AttachThreadInput(current_thread, target_thread, true);
                let _ = SetForegroundWindow(hwnd);
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }
        }
    }

    /// 用 SendInput 发 Ctrl+V(VK 方式)。
    fn send_ctrl_v() {
        unsafe {
            let mut inputs: [INPUT; 4] = [INPUT::default(); 4];

            // Ctrl down
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            // V down
            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki = KEYBDINPUT {
                wVk: VK_V,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
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

    /// 取鼠标全局坐标。
    pub fn cursor_pos() -> (i32, i32) {
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            let _ = GetCursorPos(&mut point);
            (point.x, point.y)
        }
    }

    /// 唤起面板:记录目标窗口 + 取鼠标坐标 + clamp 定位 + 显示窗口。
    pub fn show_panel(app: &tauri::AppHandle) {
        remember_target_window();

        let (cursor_x, cursor_y) = cursor_pos();
        let (screen_w, screen_h) = crate::hotkey_hook::win::screen_size();
        let (x, y) = super::compute_panel_position(cursor_x, cursor_y, screen_w, screen_h);

        if let Some(window) = app.get_webview_window("quick-paste") {
            use tauri::PhysicalPosition;
            // 鼠标坐标是物理像素,用 PhysicalPosition 直接对应。
            let _ = window.set_position(PhysicalPosition::new(x, y));
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    pub fn remember_target_window() {}
    pub fn paste_text(_content: &str) -> Result<(), String> {
        Err("不支持当前平台".to_string())
    }
    pub fn cursor_pos() -> (i32, i32) {
        (0, 0)
    }
    pub fn show_panel(_app: &tauri::AppHandle) {}
}

/// 面板尺寸常量(跨平台共享,供 clamp 计算用)。
pub const PANEL_WIDTH: i32 = 320;
pub const PANEL_HEIGHT: i32 = 360;

/// 计算面板位置:跟随鼠标但不出屏(clamp)。纯逻辑,可单测。
pub fn compute_panel_position(cursor_x: i32, cursor_y: i32, screen_w: i32, screen_h: i32) -> (i32, i32) {
    let mut x = cursor_x;
    let mut y = cursor_y;
    if x + PANEL_WIDTH > screen_w {
        x = screen_w - PANEL_WIDTH;
    }
    if y + PANEL_HEIGHT > screen_h {
        y = screen_h - PANEL_HEIGHT;
    }
    if x < 0 {
        x = 0;
    }
    if y < 0 {
        y = 0;
    }
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_clamps_when_cursor_near_bottom_right() {
        let (x, y) = compute_panel_position(1900, 1050, 1920, 1080);
        assert!(x + PANEL_WIDTH <= 1920);
        assert!(y + PANEL_HEIGHT <= 1080);
    }

    #[test]
    fn panel_stays_when_cursor_centered() {
        let (x, y) = compute_panel_position(500, 400, 1920, 1080);
        assert_eq!(x, 500);
        assert_eq!(y, 400);
    }

    #[test]
    fn panel_clamps_to_zero_for_negative_cursor() {
        let (x, y) = compute_panel_position(-50, -50, 1920, 1080);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }
}
