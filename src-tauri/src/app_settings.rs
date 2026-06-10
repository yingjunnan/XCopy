use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+V";

const SETTINGS_FILE_NAME: &str = "settings.json";
const AUTOSTART_VALUE_NAME: &str = "XCopy";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub auto_start: bool,
    pub shortcut: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_start: false,
            shortcut: DEFAULT_SHORTCUT.to_string(),
        }
    }
}

pub fn settings_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(SETTINGS_FILE_NAME)
}

pub fn load_settings_from_path(path: &Path) -> Result<AppSettings, String> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut settings: AppSettings = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    settings.shortcut = normalize_display_shortcut(&settings.shortcut)?;
    Ok(settings)
}

pub fn save_settings_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut normalized = settings.clone();
    normalized.shortcut = normalize_display_shortcut(&settings.shortcut)?;
    let content = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
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
    let shortcut = Shortcut::from_str(&normalized).map_err(|e| format!("快捷键无效：{}", e))?;

    if shortcut.mods.is_empty() {
        return Err("快捷键至少包含 Ctrl、Alt、Shift 或 Win 中的一个修饰键".to_string());
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
        run_reg_command(&[
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            AUTOSTART_VALUE_NAME,
            "/t",
            "REG_SZ",
            "/d",
            &format!("\"{}\"", exe.display()),
            "/f",
        ])
    } else {
        match run_reg_command(&[
            "delete",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            AUTOSTART_VALUE_NAME,
            "/f",
        ]) {
            Ok(()) => Ok(()),
            Err(err) if err.contains("unable to find") || err.contains("找不到") => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_platform_auto_start_enabled(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_platform_auto_start_enabled() -> Result<bool, String> {
    let output = std::process::Command::new("reg")
        .args([
            "query",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            AUTOSTART_VALUE_NAME,
        ])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(output.status.success())
}

#[cfg(not(target_os = "windows"))]
fn is_platform_auto_start_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
fn run_reg_command(args: &[&str]) -> Result<(), String> {
    let output = std::process::Command::new("reg")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(if stderr.is_empty() { stdout } else { stderr })
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
    }

    #[test]
    fn normalizes_readable_shortcut_input_for_global_registration() {
        let shortcut = normalize_shortcut_input(" Ctrl + Shift + v ").unwrap();

        assert_eq!(shortcut, "control+shift+KeyV");
    }

    #[test]
    fn rejects_shortcuts_without_modifier_keys() {
        let err = normalize_shortcut_input("V").unwrap_err();

        assert!(err.contains("至少包含"));
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
        };

        save_settings_to_path(&path, &settings).unwrap();
        let loaded = load_settings_from_path(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            loaded,
            AppSettings {
                auto_start: true,
                shortcut: "Ctrl+Alt+X".to_string(),
            }
        );
    }
}
