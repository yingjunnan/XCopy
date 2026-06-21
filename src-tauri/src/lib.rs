mod app_settings;
mod clipboard;
mod db;
mod hotkey_hook;
mod models;
mod quick_paste;
mod window_tracker;

use db::Database;
use db::RetentionPolicy;
use models::{ClipboardEntry, ClipboardFilter};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};

const TRAY_SHOW_ID: &str = "show-main-window";
const TRAY_EXIT_ID: &str = "exit-app";

/// Marker file written after the first-run window has been shown once.
/// Its absence is treated as "never launched before": we pop up the main
/// window so a freshly installed user sees the app is ready.
const FIRST_RUN_MARKER: &str = ".first_run_shown";

struct AppState {
    db: Arc<Database>,
    clipboard_state: Arc<clipboard::ClipboardState>,
    app_data_dir: PathBuf,
    settings_path: PathBuf,
    settings: Mutex<app_settings::AppSettings>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub database_bytes: u64,
    pub images_bytes: u64,
}

#[tauri::command]
fn get_history(
    state: tauri::State<AppState>,
    filter: ClipboardFilter,
) -> Result<Vec<ClipboardEntry>, String> {
    state.db.query_entries(&filter)
}

#[tauri::command]
fn delete_entry(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    if let Ok(Some(path)) = state.db.get_image_path(&id) {
        std::fs::remove_file(&path).ok();
    }
    state.db.delete_entry(&id)
}

#[tauri::command]
fn clear_history(state: tauri::State<AppState>) -> Result<(), String> {
    if let Err(e) = clipboard::remember_current_clipboard(state.clipboard_state.as_ref()) {
        eprintln!("[XCopy] failed to mark current clipboard after clear: {}", e);
    }
    state.db.clear_all()
}

#[tauri::command]
fn get_last_entry(state: tauri::State<AppState>) -> Result<Option<ClipboardEntry>, String> {
    state.db.get_last_entry()
}

#[tauri::command]
fn read_image_file(path: String) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(BASE64.encode(&bytes))
}

#[tauri::command]
fn get_storage_usage(state: tauri::State<AppState>) -> Result<StorageUsage, String> {
    let app_data_dir = &state.app_data_dir;

    // Database files: xcopy.db plus the WAL (-wal) and shared-memory (-shm) sidecars.
    let database_bytes = ["xcopy.db", "xcopy.db-wal", "xcopy.db-shm"]
        .iter()
        .map(|name| file_size(app_data_dir.join(name)))
        .sum();

    // Images directory: sum of every file under app_data_dir/images.
    let images_bytes = dir_size(&app_data_dir.join("images"));

    Ok(StorageUsage {
        database_bytes,
        images_bytes,
    })
}

fn file_size(path: PathBuf) -> u64 {
    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
}

fn dir_size(path: &Path) -> u64 {
    fn walk(dir: &Path) -> u64 {
        let mut total = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                total += walk(&entry.path());
            } else if file_type.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        total
    }
    walk(path)
}

#[tauri::command]
fn get_app_settings(state: tauri::State<AppState>) -> Result<app_settings::AppSettings, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "设置状态已锁定".to_string())?
        .clone();

    settings.auto_start = app_settings::is_auto_start_enabled().unwrap_or(settings.auto_start);
    Ok(settings)
}

#[tauri::command]
fn save_app_settings(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    settings: app_settings::AppSettings,
) -> Result<app_settings::AppSettings, String> {
    let normalized_shortcut = app_settings::normalize_display_shortcut(&settings.shortcut)?;
    let next_settings = app_settings::normalize_settings(app_settings::AppSettings {
        auto_start: settings.auto_start,
        shortcut: normalized_shortcut,
        max_history_entries: settings.max_history_entries,
        retention_days: settings.retention_days,
        quick_paste_enabled: settings.quick_paste_enabled,
        double_click_interval_ms: settings.double_click_interval_ms,
    })?;

    register_app_shortcut(&app, &next_settings.shortcut)?;
    app_settings::set_auto_start_enabled(next_settings.auto_start)?;
    // Sync the double-tap-Ctrl hook toggle without reinstalling the hook.
    hotkey_hook::win::set_enabled(next_settings.quick_paste_enabled);
    app_settings::save_settings_to_path(&state.settings_path, &next_settings)?;
    state.db.set_retention_policy(RetentionPolicy {
        max_entries: next_settings.max_history_entries,
        retention_days: next_settings.retention_days,
    })?;

    let mut stored = state
        .settings
        .lock()
        .map_err(|_| "设置状态已锁定".to_string())?;
    *stored = next_settings.clone();

    Ok(next_settings)
}

#[tauri::command]
fn hide_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

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

fn request_history_refresh(window: &tauri::WebviewWindow) {
    let _ = window.emit("window-shown", ());
    let _ = window.eval("window.__XCOPY_REFRESH_HISTORY?.();");
}

fn show_main_window(app: &tauri::AppHandle, capture_clipboard: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if capture_clipboard {
            let state = app.state::<AppState>();
            match clipboard::capture_current_clipboard(
                state.clipboard_state.as_ref(),
                state.db.as_ref(),
                &state.app_data_dir,
            ) {
                Ok(Some(entry)) => {
                    eprintln!("[XCopy] captured {}, refreshing window", entry.content_type);
                    let _ = window.emit("clipboard-update", &entry);
                }
                Ok(None) => {
                    eprintln!("[XCopy] no new clipboard entry found");
                }
                Err(e) => {
                    eprintln!("[XCopy] clipboard capture failed: {}", e);
                }
            }
        }

        let _ = window.set_skip_taskbar(false);
        let _ = window.show();
        let _ = window.set_focus();
        request_history_refresh(&window);
        eprintln!("[XCopy] window shown and refresh requested");
    } else {
        eprintln!("[XCopy] ERROR: main window not found");
    }
}

fn register_app_shortcut(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let registration_shortcut = app_settings::normalize_shortcut_input(shortcut)?;
    let shortcuts = app.global_shortcut();

    if let Err(e) = shortcuts.unregister_all() {
        eprintln!("[XCopy] failed to unregister previous shortcuts: {}", e);
    }

    shortcuts
        .on_shortcut(registration_shortcut.as_str(), |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                eprintln!("[XCopy] shortcut pressed, showing window");
                show_main_window(app, true);
            }
        })
        .map_err(|e| format!("注册快捷键失败：{}", e))
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, TRAY_SHOW_ID, "显示主界面", true, None::<&str>)?;
    let exit_item = MenuItem::with_id(app, TRAY_EXIT_ID, "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &separator, &exit_item])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("XCopy")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => show_main_window(app, true),
            TRAY_EXIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle(), true);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let settings_path = app_settings::settings_path(&app_data_dir);
            // Marker for the first-run onboarding window; resolve it now, before
            // app_data_dir is moved into the clipboard monitor thread below.
            let first_run_marker = app_data_dir.join(FIRST_RUN_MARKER);
            let mut settings = app_settings::load_settings_from_path(&settings_path)
                .unwrap_or_else(|e| {
                    eprintln!("[XCopy] failed to load settings, using defaults: {}", e);
                    app_settings::AppSettings::default()
                });
            settings.auto_start =
                app_settings::is_auto_start_enabled().unwrap_or(settings.auto_start);

            let db = Arc::new(Database::new(app_data_dir.clone()).expect("Failed to init DB"));
            db.set_retention_policy(RetentionPolicy {
                max_entries: settings.max_history_entries,
                retention_days: settings.retention_days,
            })?;
            let db_clone = db.clone();
            let clipboard_state = Arc::new(clipboard::ClipboardState::default());
            let clipboard_state_clone = clipboard_state.clone();

            app.manage(AppState {
                db,
                clipboard_state,
                app_data_dir: app_data_dir.clone(),
                settings_path,
                settings: Mutex::new(settings.clone()),
            });

            // Start clipboard monitor in background thread
            let handle = app.handle().clone();
            clipboard::start_clipboard_monitor(
                handle,
                db_clone,
                clipboard_state_clone,
                app_data_dir,
            );

            register_app_shortcut(app.handle(), &settings.shortcut)?;
            setup_tray(app)?;

            // Low-level keyboard hook: detect double-tap Ctrl to summon the
            // quick-paste panel. Runs on its own thread with a message pump.
            hotkey_hook::win::install(
                app.handle().clone(),
                settings.double_click_interval_ms,
                settings.quick_paste_enabled,
            );

            // Double-tap Ctrl fires "quick-paste-trigger" from the hook thread;
            // summon the panel on the main thread.
            let trigger_handle = app.handle().clone();
            app.listen("quick-paste-trigger", move |_event| {
                quick_paste::win::show_panel(&trigger_handle);
            });

            // First-run onboarding: the very first time the app launches after
            // install (marker file absent), pop up the main window once so the
            // user sees it's ready. Subsequent launches stay hidden as usual.
            if !first_run_marker.exists() {
                eprintln!("[XCopy] first run detected, showing main window");
                show_main_window(app.handle(), false);
                if let Err(e) = std::fs::write(&first_run_marker, "") {
                    eprintln!(
                        "[XCopy] failed to write first-run marker at {}: {}",
                        first_run_marker.display(),
                        e
                    );
                }
            }

            Ok(())
        })
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
        .on_window_event(|window, event| {
            // The main popup hides shortly after losing focus (its normal UX).
            // The quick-paste panel hides immediately on losing focus (no delay,
            // per its spec) so clicking outside dismisses it at once.
            // Other windows (e.g. the image preview) are left alone.
            if let tauri::WindowEvent::Focused(false) = event {
                let label = window.label().to_string();
                if label == "main" {
                    let handle = window.app_handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(150));
                        if let Some(w) = handle.get_webview_window(&label) {
                            if let Ok(false) = w.is_focused() {
                                let _ = w.hide();
                            }
                        }
                    });
                } else if label == "quick-paste" {
                    // Immediate hide, no delay — clicking outside should dismiss
                    // the lightweight panel at once.
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
