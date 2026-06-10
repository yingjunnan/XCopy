#[cfg(target_os = "windows")]
pub mod win {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowTextLengthW};
    use windows::Win32::Foundation::HWND;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

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
}

#[cfg(not(target_os = "windows"))]
pub mod win {
    pub fn get_active_window_title() -> String {
        String::from("Unknown")
    }
}
