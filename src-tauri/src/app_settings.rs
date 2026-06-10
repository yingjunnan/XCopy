use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+V";
pub const DEFAULT_MAX_HISTORY_ENTRIES: usize = 1000;
pub const DEFAULT_RETENTION_DAYS: i64 = 30;
pub const MIN_HISTORY_ENTRIES: usize = 1;
pub const MAX_HISTORY_ENTRIES: usize = 100_000;
pub const MIN_RETENTION_DAYS: i64 = 1;
pub const MAX_RETENTION_DAYS: i64 = 3650;

const SETTINGS_FILE_NAME: &str = "settings.json";
const AUTOSTART_VALUE_NAME: &str = "XCopy";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_start: bool,
    pub shortcut: String,
    #[serde(default = "default_max_history_entries")]
    pub max_history_entries: usize,
    #[serde(default = "default_retention_days")]
    pub retention_days: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
            max_history_entries: DEFAULT_MAX_HISTORY_ENTRIES,
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

fn default_max_history_entries() -> usize {
    DEFAULT_MAX_HISTORY_ENTRIES
}

fn default_retention_days() -> i64 {
    DEFAULT_RETENTION_DAYS
}

pub fn settings_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(SETTINGS_FILE_NAME)
}

pub fn load_settings_from_path(path: &Path) -> Result<AppSettings, String> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let settings: AppSettings = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    normalize_settings(settings)
}

pub fn save_settings_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let normalized = normalize_settings(settings.clone())?;
    let content = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

pub fn normalize_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    settings.shortcut = normalize_display_shortcut(&settings.shortcut)?;
    settings.max_history_entries = settings
        .max_history_entries
        .clamp(MIN_HISTORY_ENTRIES, MAX_HISTORY_ENTRIES);
    settings.retention_days = settings
        .retention_days
        .clamp(MIN_RETENTION_DAYS, MAX_RETENTION_DAYS);
    Ok(settings)
}

pub fn normalize_shortcut_input(input: &str) -> Result<String, String> {
    let shortcut = parse_shortcut(input)?;
    Ok(registration_shortcut(&shortcut))
}

pub fn normalize_display_shortcut(input: &str) -> Result<String, String> {
    let shortcut = parse_shortcut(input)?;
    Ok(display_shortcut(&shortcut))
}

pub fn set_auto_start_enabled(enabled: bool) -> Result<(), String> {
    set_platform_auto_start_enabled(enabled)
}

pub fn is_auto_start_enabled() -> Result<bool, String> {
    is_platform_auto_start_enabled()
}

fn parse_shortcut(input: &str) -> Result<Shortcut, String> {
    let normalized = normalize_aliases(input);
    let shortcut = Shortcut::from_str(&normalized).map_err(|e| format!("Invalid shortcut: {}", e))?;

    if shortcut.mods.is_empty() {
        return Err("Shortcut must include Ctrl, Alt, Shift, or Win".to_string());
    }

    Ok(shortcut)
}

fn normalize_aliases(input: &str) -> String {
    input
        .split('+')
        .map(|part| match part.trim().to_ascii_lowercase().as_str() {
            "win" | "windows" | "meta" => "super".to_string(),
            "cmd" | "command" => "super".to_string(),
            "control" => "ctrl".to_string(),
            value => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn registration_shortcut(shortcut: &Shortcut) -> String {
    let mut parts = modifier_parts(shortcut);
    parts.push(shortcut.key.to_string());
    parts.join("+")
}

fn display_shortcut(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();

    if shortcut.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    if shortcut.mods.contains(Modifiers::SUPER) {
        parts.push("Win".to_string());
    }

    parts.push(display_key(shortcut.key.to_string()));
    parts.join("+")
}

fn modifier_parts(shortcut: &Shortcut) -> Vec<String> {
    let mut parts = Vec::new();

    if shortcut.mods.contains(Modifiers::CONTROL) {
        parts.push("control".to_string());
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        parts.push("alt".to_string());
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        parts.push("shift".to_string());
    }
    if shortcut.mods.contains(Modifiers::SUPER) {
        parts.push("super".to_string());
    }

    parts
}

fn display_key(key: String) -> String {
    if let Some(letter) = key.strip_prefix("Key") {
        return letter.to_string();
    }

    if let Some(digit) = key.strip_prefix("Digit") {
        return digit.to_string();
    }

    key
}

#[cfg(target_os = "windows")]
fn set_platform_auto_start_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        set_run_key_value(AUTOSTART_VALUE_NAME, &format!("\"{}\"", exe.display()))
    } else {
        delete_run_key_value(AUTOSTART_VALUE_NAME)
    }
}

#[cfg(not(target_os = "windows"))]
fn set_platform_auto_start_enabled(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_platform_auto_start_enabled() -> Result<bool, String> {
    run_key_value_exists(AUTOSTART_VALUE_NAME)
}

#[cfg(not(target_os = "windows"))]
fn is_platform_auto_start_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

#[cfg(target_os = "windows")]
fn set_run_key_value(name: &str, value: &str) -> Result<(), String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_SZ,
    };

    unsafe {
        let mut key = HKEY::default();
        let key_path = pcwstr_from_str(RUN_KEY_PATH);
        let status = RegCreateKeyW(HKEY_CURRENT_USER, key_path.as_pcwstr(), &mut key);
        win32_status(status, "open Run registry key")?;

        let value_name = pcwstr_from_str(name);
        let value_wide = wide_null(value);
        let bytes = std::slice::from_raw_parts(
            value_wide.as_ptr().cast::<u8>(),
            value_wide.len() * std::mem::size_of::<u16>(),
        );
        let status = RegSetValueExW(key, value_name.as_pcwstr(), 0, REG_SZ, Some(bytes));
        let close_status = RegCloseKey(key);

        win32_status(status, "set autostart registry value")?;
        if close_status != ERROR_SUCCESS {
            win32_status(close_status, "close Run registry key")?;
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn delete_run_key_value(name: &str) -> Result<(), String> {
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    unsafe {
        let mut key = HKEY::default();
        let key_path = pcwstr_from_str(RUN_KEY_PATH);
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_pcwstr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        );
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        win32_status(status, "open Run registry key")?;

        let value_name = pcwstr_from_str(name);
        let status = RegDeleteValueW(key, value_name.as_pcwstr());
        let close_status = RegCloseKey(key);

        if status != ERROR_FILE_NOT_FOUND {
            win32_status(status, "delete autostart registry value")?;
        }
        if close_status != ERROR_SUCCESS {
            win32_status(close_status, "close Run registry key")?;
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn run_key_value_exists(name: &str) -> Result<bool, String> {
    use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
    };

    unsafe {
        let mut key = HKEY::default();
        let key_path = pcwstr_from_str(RUN_KEY_PATH);
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_pcwstr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        win32_status(status, "open Run registry key")?;

        let value_name = pcwstr_from_str(name);
        let status = RegQueryValueExW(key, value_name.as_pcwstr(), None, None, None, None);
        let close_status = RegCloseKey(key);

        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        win32_status(status, "query autostart registry value")?;
        win32_status(close_status, "close Run registry key")?;

        Ok(true)
    }
}

#[cfg(target_os = "windows")]
struct WideString(Vec<u16>);

#[cfg(target_os = "windows")]
impl WideString {
    fn as_pcwstr(&self) -> windows::core::PCWSTR {
        windows::core::PCWSTR(self.0.as_ptr())
    }
}

#[cfg(target_os = "windows")]
fn pcwstr_from_str(value: &str) -> WideString {
    WideString(wide_null(value))
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn win32_status(
    status: windows::Win32::Foundation::WIN32_ERROR,
    action: &str,
) -> Result<(), String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;

    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("{} failed with code {}", action, status.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_settings_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "xcopy-settings-test-{}-{}.json",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn default_settings_use_startup_disabled_and_ctrl_shift_v() {
        let settings = AppSettings::default();

        assert!(!settings.auto_start);
        assert_eq!(settings.shortcut, DEFAULT_SHORTCUT);
        assert_eq!(settings.max_history_entries, DEFAULT_MAX_HISTORY_ENTRIES);
        assert_eq!(settings.retention_days, DEFAULT_RETENTION_DAYS);
    }

    #[test]
    fn normalizes_readable_shortcut_input_for_global_registration() {
        let shortcut = normalize_shortcut_input(" Ctrl + Shift + v ").unwrap();

        assert_eq!(shortcut, "control+shift+KeyV");
    }

    #[test]
    fn rejects_shortcuts_without_modifier_keys() {
        let err = normalize_shortcut_input("V").unwrap_err();

        assert!(err.contains("must include"));
    }

    #[test]
    fn loads_defaults_when_settings_file_is_missing() {
        let path = temp_settings_path("missing");

        let settings = load_settings_from_path(&path).unwrap();

        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn saves_and_loads_settings_roundtrip() {
        let path = temp_settings_path("roundtrip");
        let settings = AppSettings {
            auto_start: true,
            shortcut: "control+alt+KeyX".to_string(),
            max_history_entries: 250,
            retention_days: 14,
        };

        save_settings_to_path(&path, &settings).unwrap();
        let loaded = load_settings_from_path(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            loaded,
            AppSettings {
                auto_start: true,
                shortcut: "Ctrl+Alt+X".to_string(),
                max_history_entries: 250,
                retention_days: 14,
            }
        );
    }

    #[test]
    fn loads_older_settings_with_default_retention_values() {
        let path = temp_settings_path("legacy");
        std::fs::write(
            &path,
            r#"{"autoStart":true,"shortcut":"Ctrl+Shift+V"}"#,
        )
        .unwrap();

        let loaded = load_settings_from_path(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.max_history_entries, DEFAULT_MAX_HISTORY_ENTRIES);
        assert_eq!(loaded.retention_days, DEFAULT_RETENTION_DAYS);
    }

    #[test]
    fn clamps_retention_settings_to_supported_range() {
        let settings = normalize_settings(AppSettings {
            auto_start: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
            max_history_entries: 0,
            retention_days: 0,
        })
        .unwrap();

        assert_eq!(settings.max_history_entries, MIN_HISTORY_ENTRIES);
        assert_eq!(settings.retention_days, MIN_RETENTION_DAYS);
    }
}
