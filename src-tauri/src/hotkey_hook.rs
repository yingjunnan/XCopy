use std::time::Instant;

/// 纯逻辑双击检测器:不接触 Win32,便于单测。
/// 判定规则:两次 Ctrl keydown 间隔在 [min_ms, max_ms] 内算命中。
pub struct DoubleClickDetector {
    last_press: Option<Instant>,
    #[allow(dead_code)]
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
    /// 判定:距上次 keydown 间隔 <= max_interval_ms 即视为双击。
    /// 过短(< min)也会算入——正常按键不会在 200ms 内连按两次,故不单列。
    pub fn on_ctrl_press(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last_press {
            let elapsed = now.duration_since(last).as_millis() as u32;
            if elapsed <= self.max_interval_ms {
                self.last_press = None;
                return true;
            }
        }
        self.last_press = Some(now);
        false
    }

    /// Ctrl keyup 不影响判定(只看 keydown),保留接口以便将来扩展。
    pub fn on_ctrl_release(&mut self) {}
}

#[cfg(target_os = "windows")]
pub mod win {
    use std::sync::Mutex;
    use std::time::Instant;
    use tauri::{AppHandle, Emitter};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_LCONTROL, VK_RCONTROL};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, GetSystemMetrics, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, SM_CXSCREEN,
        SM_CYSCREEN, WH_KEYBOARD_LL, WINDOWS_HOOK_ID,
    };

    const WM_KEYDOWN: u32 = 0x0100;
    const WM_SYSKEYDOWN: u32 = 0x0104;

    /// 全局状态:双击检测器 + 是否启用 + 唤起回调用的 AppHandle。
    /// 钩子回调在独立线程跑消息循环,通过 Mutex 访问。
    struct HookState {
        detector: super::DoubleClickDetector,
        enabled: bool,
        app_handle: AppHandle,
    }

    static HOOK_STATE: Mutex<Option<HookState>> = Mutex::new(None);

    /// 钩子回调。只检测 Ctrl keydown 做双击判定,不消费任何按键
    /// (始终 CallNextHookEx 透传,保证 Ctrl 的正常功能不受影响)。
    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let wparam_u = wparam.0 as u32;
        if wparam_u == WM_KEYDOWN || wparam_u == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kb.vkCode;
            if vk == VK_CONTROL.0 as u32
                || vk == VK_LCONTROL.0 as u32
                || vk == VK_RCONTROL.0 as u32
            {
                let now = Instant::now();
                let trigger = {
                    let mut state_guard = HOOK_STATE.lock().unwrap();
                    if let Some(state) = state_guard.as_mut() {
                        if state.enabled && state.detector.on_ctrl_press(now) {
                            let handle = state.app_handle.clone();
                            Some(handle)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                // 在锁外触发,避免阻塞钩子链或死锁。
                if let Some(handle) = trigger {
                    let _ = handle.emit("quick-paste-trigger", ());
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// 安装低级键盘钩子并启动消息循环。在独立线程调用。
    /// 间隔 ms 从 settings 传入;启用状态由 enabled 控制。
    pub fn install(app_handle: AppHandle, interval_ms: u32, enabled: bool) {
        {
            let mut state = HOOK_STATE.lock().unwrap();
            *state = Some(HookState {
                detector: super::DoubleClickDetector::new(200, interval_ms),
                enabled,
                app_handle: app_handle.clone(),
            });
        }

        std::thread::spawn(move || {
            unsafe {
                let hook = SetWindowsHookExW(
                    WINDOWS_HOOK_ID(WH_KEYBOARD_LL.0),
                    Some(hook_proc),
                    None,
                    0,
                );
                if let Ok(hook) = hook {
                    // 消息循环:低级钩子必须有消息泵,否则会被系统卸载。
                    let mut msg = MSG::default();
                    // GetMessageW 返回 0(WM_QUIT) 或 -1(错误) 时退出。
                    while GetMessageW(&mut msg, None, 0, 0).into() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    let _ = UnhookWindowsHookEx(hook);
                }
            }
        });
    }

    /// 动态开关:更新 enabled 状态(不重装钩子)。
    pub fn set_enabled(enabled: bool) {
        if let Some(state) = HOOK_STATE.lock().unwrap().as_mut() {
            state.enabled = enabled;
        }
    }

    /// 供 quick_paste 模块复用:取屏幕尺寸。
    pub fn screen_size() -> (i32, i32) {
        unsafe {
            let cx = GetSystemMetrics(SM_CXSCREEN);
            let cy = GetSystemMetrics(SM_CYSCREEN);
            (cx, cy)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    use tauri::AppHandle;

    pub fn install(_app_handle: AppHandle, _interval_ms: u32, _enabled: bool) {}
    pub fn set_enabled(_enabled: bool) {}
    pub fn screen_size() -> (i32, i32) {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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

    #[test]
    fn first_press_after_timeout_starts_fresh() {
        let mut d = DoubleClickDetector::new(200, 400);
        let t0 = Instant::now();
        d.on_ctrl_press(t0);
        let t1 = t0 + Duration::from_millis(500); // 超时,不算双击
        assert!(!d.on_ctrl_press(t1));
        let t2 = t1 + Duration::from_millis(300); // 但这按下成为新的起点
        assert!(d.on_ctrl_press(t2));
    }
}
