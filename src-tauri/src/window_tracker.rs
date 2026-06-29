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
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::System::Threading::OpenProcess;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetForegroundWindow, GetIconInfo, GetWindowTextW, GetWindowTextLengthW,
        GetWindowThreadProcessId, ICONINFO,
    };

    // PROCESS_QUERY_LIMITED_INFORMATION lives under Win32_System_Threading but is
    // only re-exported at the crate root of that module in 0.58; pull it in here
    // so the OpenProcess call reads cleanly.
    use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

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
        use windows::Win32::System::Threading::{PROCESS_NAME_WIN32, QueryFullProcessImageNameW};
        unsafe {
            let mut buf = vec![0u16; 1024];
            let mut size = buf.len() as u32;
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
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
        let end = shfi
            .szDisplayName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(shfi.szDisplayName.len());
        if end == 0 {
            return None;
        }
        Some(
            OsString::from_wide(&shfi.szDisplayName[..end])
                .to_string_lossy()
                .into_owned(),
        )
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
        app_data_dir
            .join("app_icons")
            .join(format!("{}.png", hash))
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

            // The color bitmap gives us the RGBA pixels (often without a real
            // alpha channel on older icons); the mask supplies transparency.
            let color = info.hbmColor;
            let mask = info.hbmMask;
            let result = (|| {
                if color.is_invalid() {
                    return None;
                }
                let (rgba, width, height) = bitmap_to_rgba(color)?;
                // mask_alpha returns None when the mask is unusable; treat that
                // as fully opaque (keep the color bitmap's alpha as-is).
                let alpha = mask_alpha(mask, width, height)
                    .unwrap_or_else(|| vec![255u8; width * height]);
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
            let copied = GetDIBits(hdc, bmp, 0, 0, None, &mut bi, DIB_RGB_COLORS);
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

            // 1bpp mask: each bit, 0 = opaque, 1 = transparent. Rows padded to 4 bytes.
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

    /// Overlay mask alpha onto the color RGBA. If the color already has real
    /// alpha (non-zero), keep it; otherwise use the mask.
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
