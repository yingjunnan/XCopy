use arboard::Clipboard;
use chrono::Utc;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::db::Database;
use crate::models::ClipboardEntry;
use crate::window_tracker::win;

fn truncate_preview(text: &str, max_len: usize) -> String {
    let preview: String = text.chars().take(max_len).collect();
    if text.chars().count() > max_len {
        format!("{}...", preview)
    } else {
        preview
    }
}

fn detect_content_type(text: &str) -> &str {
    let trimmed = text.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return "link";
    }
    if trimmed.contains('@') && trimmed.contains('.') && !trimmed.contains(' ') {
        return "link";
    }
    "text"
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClipboardSignature {
    content_type: &'static str,
    hash: u64,
}

#[derive(Default)]
pub struct ClipboardState {
    last_seen: Mutex<Option<ClipboardSignature>>,
}

impl ClipboardState {
    fn remember(&self, signature: ClipboardSignature) {
        if let Ok(mut last_seen) = self.last_seen.lock() {
            *last_seen = Some(signature);
        }
    }

    fn mark_if_changed(&self, signature: ClipboardSignature) -> bool {
        let Ok(mut last_seen) = self.last_seen.lock() else {
            return false;
        };

        if last_seen.as_ref() == Some(&signature) {
            return false;
        }

        *last_seen = Some(signature);
        true
    }
}

fn hash_value<T: Hash>(value: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn text_signature(text: &str) -> ClipboardSignature {
    ClipboardSignature {
        content_type: "text",
        hash: hash_value(text),
    }
}

fn image_signature(bytes: &[u8], width: usize, height: usize) -> ClipboardSignature {
    let mut hasher = DefaultHasher::new();
    "image".hash(&mut hasher);
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    bytes.hash(&mut hasher);

    ClipboardSignature {
        content_type: "image",
        hash: hasher.finish(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_signature_is_not_captured_again() {
        let state = ClipboardState::default();
        let signature = ClipboardSignature {
            content_type: "text",
            hash: 42,
        };

        state.remember(signature);

        assert!(!state.mark_if_changed(signature));
    }

    #[test]
    fn changed_signature_is_captured_once() {
        let state = ClipboardState::default();
        let signature = ClipboardSignature {
            content_type: "text",
            hash: 42,
        };

        assert!(state.mark_if_changed(signature));
        assert!(!state.mark_if_changed(signature));
    }
}

pub fn start_clipboard_monitor(
    app_handle: AppHandle,
    db: std::sync::Arc<Database>,
    clipboard_state: std::sync::Arc<ClipboardState>,
    app_data_dir: PathBuf,
) {
    std::thread::spawn(move || {
        let mut clipboard = Clipboard::new().expect("Failed to create clipboard");

        loop {
            std::thread::sleep(Duration::from_millis(500));

            match capture_from_clipboard(
                &mut clipboard,
                clipboard_state.as_ref(),
                db.as_ref(),
                &app_data_dir,
            ) {
                Ok(Some(entry)) => {
                    eprintln!(
                        "[XCopy] inserted {}, emitting clipboard-update",
                        entry.content_type
                    );
                    let _ = app_handle.emit("clipboard-update", &entry);
                }
                Ok(None) => {}
                Err(e) => eprintln!("[XCopy] clipboard monitor error: {}", e),
            }
        }
    });
}

pub fn capture_current_clipboard(
    clipboard_state: &ClipboardState,
    db: &Database,
    app_data_dir: &Path,
) -> Result<Option<ClipboardEntry>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    capture_from_clipboard(&mut clipboard, clipboard_state, db, app_data_dir)
}

pub fn remember_current_clipboard(clipboard_state: &ClipboardState) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    if let Some(signature) = current_signature(&mut clipboard) {
        clipboard_state.remember(signature);
    }
    Ok(())
}

fn capture_from_clipboard(
    clipboard: &mut Clipboard,
    clipboard_state: &ClipboardState,
    db: &Database,
    app_data_dir: &Path,
) -> Result<Option<ClipboardEntry>, String> {
    let source_app = win::get_active_window_title();
    let now = Utc::now().to_rfc3339();

    if let Ok(image_data) = clipboard.get_image() {
        let img = image_data.to_owned();
        if img.width == 0 {
            return Ok(None);
        }

        let signature = image_signature(&img.bytes, img.width as usize, img.height as usize);
        if !clipboard_state.mark_if_changed(signature) {
            return Ok(None);
        }

        let images_dir = app_data_dir.join("images");
        fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

        let id = Uuid::new_v4().to_string();
        let filepath = images_dir.join(format!("{}.png", id));
        let png_bytes = crate::png_encode::rgba_to_png(&img.bytes, img.width as usize, img.height as usize)?;
        fs::write(&filepath, &png_bytes).map_err(|e| e.to_string())?;

        let entry = ClipboardEntry {
            id,
            content_type: "image".to_string(),
            content: format!("Image {}x{}", img.width, img.height),
            source_app,
            source_app_icon: None,
            preview: format!("{}x{}px image", img.width, img.height),
            created_at: now,
            image_path: Some(filepath.to_string_lossy().to_string()),
        };

        if db.insert_entry_if_changed(&entry)? {
            return Ok(Some(entry));
        }

        let _ = fs::remove_file(filepath);
        return Ok(None);
    }

    if let Ok(text) = clipboard.get_text() {
        if text.trim().is_empty() {
            return Ok(None);
        }

        let signature = text_signature(&text);
        if !clipboard_state.mark_if_changed(signature) {
            return Ok(None);
        }

        let entry = ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: detect_content_type(&text).to_string(),
            content: text.clone(),
            source_app,
            source_app_icon: None,
            preview: truncate_preview(&text, 100),
            created_at: now,
            image_path: None,
        };

        if db.insert_entry_if_changed(&entry)? {
            return Ok(Some(entry));
        }
    }

    Ok(None)
}

fn current_signature(clipboard: &mut Clipboard) -> Option<ClipboardSignature> {
    if let Ok(image_data) = clipboard.get_image() {
        let img = image_data.to_owned();
        if img.width > 0 {
            return Some(image_signature(
                &img.bytes,
                img.width as usize,
                img.height as usize,
            ));
        }
    }

    if let Ok(text) = clipboard.get_text() {
        if !text.trim().is_empty() {
            return Some(text_signature(&text));
        }
    }

    None
}
