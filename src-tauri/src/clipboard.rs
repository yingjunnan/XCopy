use arboard::Clipboard;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
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

pub fn start_clipboard_monitor(
    app_handle: AppHandle,
    db: std::sync::Arc<Database>,
    app_data_dir: PathBuf,
) {
    std::thread::spawn(move || {
        let mut clipboard = Clipboard::new().expect("Failed to create clipboard");

        loop {
            std::thread::sleep(Duration::from_millis(500));

            match capture_from_clipboard(&mut clipboard, db.as_ref(), &app_data_dir) {
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
    db: &Database,
    app_data_dir: &Path,
) -> Result<Option<ClipboardEntry>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    capture_from_clipboard(&mut clipboard, db, app_data_dir)
}

fn capture_from_clipboard(
    clipboard: &mut Clipboard,
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

        let images_dir = app_data_dir.join("images");
        fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

        let id = Uuid::new_v4().to_string();
        let filepath = images_dir.join(format!("{}.png", id));
        let png_bytes = bgra_to_png(&img.bytes, img.width as usize, img.height as usize)?;
        fs::write(&filepath, &png_bytes).map_err(|e| e.to_string())?;

        let entry = ClipboardEntry {
            id,
            content_type: "image".to_string(),
            content: format!("Image {}x{}", img.width, img.height),
            source_app,
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

        let entry = ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: detect_content_type(&text).to_string(),
            content: text.clone(),
            source_app,
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

/// Simple BGRA to PNG conversion using a minimal encoder
fn bgra_to_png(bytes: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let mut rgba = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks(4) {
        if chunk.len() == 4 {
            rgba.push(chunk[2]); // R
            rgba.push(chunk[1]); // G
            rgba.push(chunk[0]); // B
            rgba.push(chunk[3]); // A
        }
    }

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr_data.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr_data.push(8);
    ihdr_data.push(6);
    ihdr_data.push(0);
    ihdr_data.push(0);
    ihdr_data.push(0);
    write_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // IDAT
    let mut raw_data = Vec::with_capacity(height + rgba.len());
    for y in 0..height {
        raw_data.push(0);
        let start = y * width * 4;
        let end = start + width * 4;
        raw_data.extend_from_slice(&rgba[start..end.min(rgba.len())]);
    }

    let compressed = deflate(&raw_data);
    write_png_chunk(&mut png, b"IDAT", &compressed);
    write_png_chunk(&mut png, b"IEND", &[]);

    Ok(png)
}

fn write_png_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);
    let crc = crc32(chunk_type, data);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc_table = [0u32; 256];
    for (i, entry) in crc_table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xedb88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        *entry = c;
    }

    let mut crc: u32 = 0xffffffff;
    for &b in chunk_type.iter().chain(data.iter()) {
        let idx = ((crc as u8) ^ b) as usize;
        crc = crc_table[idx] ^ (crc >> 8);
    }
    !crc
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.push(0x78);
    output.push(0x01);

    let mut pos = 0;
    while pos < data.len() {
        let remaining = data.len() - pos;
        let block_size = remaining.min(65535);
        let is_final = pos + block_size >= data.len();

        output.push(if is_final { 0x01 } else { 0x00 });
        output.extend_from_slice(&(block_size as u16).to_le_bytes());
        output.extend_from_slice(&(!(block_size as u16)).to_le_bytes());
        output.extend_from_slice(&data[pos..pos + block_size]);
        pos += block_size;
    }

    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    output.extend_from_slice(&adler.to_be_bytes());

    output
}
