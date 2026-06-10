mod clipboard;
mod db;
mod models;
mod window_tracker;

use db::Database;
use models::{ClipboardEntry, ClipboardFilter};
use std::sync::Arc;
use tauri::{Emitter, Manager};

struct AppState {
    db: Arc<Database>,
}

#[tauri::command]
fn get_history(state: tauri::State<AppState>, filter: ClipboardFilter) -> Result<Vec<ClipboardEntry>, String> {
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
    state.db.clear_all()
}

#[tauri::command]
fn get_last_entry(state: tauri::State<AppState>) -> Result<Option<ClipboardEntry>, String> {
    state.db.get_last_entry()
}

#[tauri::command]
fn read_image_file(path: String) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(BASE64.encode(&bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        eprintln!("[XCopy] shortcut pressed, showing window");
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.set_skip_taskbar(false);
                            let _ = window.show();
                            let _ = window.set_focus();
                            // Notify frontend to refresh data
                            let _ = app.emit("window-shown", ());
                            eprintln!("[XCopy] window-shown event emitted");
                        } else {
                            eprintln!("[XCopy] ERROR: main window not found");
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let db = Arc::new(Database::new(app_data_dir.clone()).expect("Failed to init DB"));
            let db_clone = db.clone();

            app.manage(AppState { db });

            // Start clipboard monitor in background thread
            let handle = app.handle().clone();
            clipboard::start_clipboard_monitor(handle, db_clone, app_data_dir);

            // Register global shortcut: Ctrl+Shift+V
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV);
            app.global_shortcut().register(shortcut)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            delete_entry,
            clear_history,
            get_last_entry,
            read_image_file,
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
