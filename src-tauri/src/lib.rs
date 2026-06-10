mod clipboard;
mod db;
mod models;
mod window_tracker;

use db::Database;
use models::{ClipboardEntry, ClipboardFilter};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

const TRAY_SHOW_ID: &str = "show-main-window";
const TRAY_EXIT_ID: &str = "exit-app";

struct AppState {
    db: Arc<Database>,
    clipboard_state: Arc<clipboard::ClipboardState>,
    app_data_dir: PathBuf,
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
fn hide_main_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
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
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        eprintln!("[XCopy] shortcut pressed, showing window");
                        show_main_window(app, true);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let db = Arc::new(Database::new(app_data_dir.clone()).expect("Failed to init DB"));
            let db_clone = db.clone();
            let clipboard_state = Arc::new(clipboard::ClipboardState::default());
            let clipboard_state_clone = clipboard_state.clone();

            app.manage(AppState {
                db,
                clipboard_state,
                app_data_dir: app_data_dir.clone(),
            });

            // Start clipboard monitor in background thread
            let handle = app.handle().clone();
            clipboard::start_clipboard_monitor(
                handle,
                db_clone,
                clipboard_state_clone,
                app_data_dir,
            );

            // Register global shortcut: Ctrl+Shift+V
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
            app.global_shortcut().register(shortcut)?;
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            delete_entry,
            clear_history,
            get_last_entry,
            read_image_file,
            hide_main_window,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                // Delay hiding to avoid conflict with drag start
                let label = window.label().to_string();
                let handle = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    if let Some(w) = handle.get_webview_window(&label) {
                        if let Ok(false) = w.is_focused() {
                            let _ = w.hide();
                        }
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
