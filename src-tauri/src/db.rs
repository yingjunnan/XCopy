use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::app_settings::{DEFAULT_MAX_HISTORY_ENTRIES, DEFAULT_RETENTION_DAYS};
use crate::models::{ClipboardEntry, ClipboardFilter};

#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    pub max_entries: usize,
    pub retention_days: i64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_HISTORY_ENTRIES,
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

pub struct Database {
    conn: Mutex<Connection>,
    app_data_dir: PathBuf,
    retention_policy: Mutex<RetentionPolicy>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_db() -> Database {
        let dir = std::env::temp_dir().join(format!("xcopy-test-{}", Uuid::new_v4()));
        Database::new(dir).expect("test database should open")
    }

    fn text_entry(content: &str) -> ClipboardEntry {
        ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: "text".to_string(),
            content: content.to_string(),
            source_app: "test".to_string(),
            source_app_icon: None,
            preview: content.to_string(),
            image_path: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    fn text_entry_at(content: &str, created_at: &str) -> ClipboardEntry {
        ClipboardEntry {
            created_at: created_at.to_string(),
            ..text_entry(content)
        }
    }

    fn image_entry(path: PathBuf) -> ClipboardEntry {
        ClipboardEntry {
            id: Uuid::new_v4().to_string(),
            content_type: "image".to_string(),
            content: "Image 1x1".to_string(),
            source_app: "test".to_string(),
            source_app_icon: None,
            preview: "1x1px image".to_string(),
            image_path: Some(path.to_string_lossy().to_string()),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn insert_entry_if_changed_skips_duplicate_latest_content() {
        let db = test_db();
        let first = text_entry("copied text");
        let duplicate = text_entry("copied text");
        let different = text_entry("different text");

        assert!(db.insert_entry_if_changed(&first).unwrap());
        assert!(!db.insert_entry_if_changed(&duplicate).unwrap());
        assert!(db.insert_entry_if_changed(&different).unwrap());

        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "different text");
        assert_eq!(entries[1].content, "copied text");
    }

    #[test]
    fn retention_policy_prunes_history_to_configured_max_entries() {
        let db = test_db();
        db.set_retention_policy(RetentionPolicy {
            max_entries: 3,
            retention_days: 30,
        })
        .unwrap();

        for i in 0..4 {
            let created_at = (Utc::now() + ChronoDuration::seconds(i)).to_rfc3339();
            db.insert_entry(&text_entry_at(&format!("entry {}", i), &created_at))
                .unwrap();
        }

        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();

        assert_eq!(entries.len(), 3);
        assert!(!entries.iter().any(|entry| entry.content == "entry 0"));
    }

    #[test]
    fn retention_policy_prunes_entries_older_than_configured_days() {
        let db = test_db();
        db.set_retention_policy(RetentionPolicy {
            max_entries: 100,
            retention_days: 2,
        })
        .unwrap();
        let old_date = (Utc::now() - ChronoDuration::days(3)).to_rfc3339();
        let recent_date = Utc::now().to_rfc3339();

        db.insert_entry(&text_entry_at("old", &old_date)).unwrap();
        db.insert_entry(&text_entry_at("recent", &recent_date)).unwrap();

        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "recent");
    }

    #[test]
    fn retention_pruning_removes_pruned_image_files() {
        let db = test_db();
        db.set_retention_policy(RetentionPolicy {
            max_entries: 1,
            retention_days: 30,
        })
        .unwrap();
        let images_dir = db.app_data_dir.join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        let old_image_path = images_dir.join("old.png");
        std::fs::write(&old_image_path, b"png").unwrap();

        let old_entry = ClipboardEntry {
            created_at: "2026-01-01T00:00:00+00:00".to_string(),
            ..image_entry(old_image_path.clone())
        };
        db.insert_entry(&old_entry).unwrap();
        db.insert_entry(&text_entry_at("new", "2026-01-01T00:00:01+00:00"))
            .unwrap();

        assert!(!old_image_path.exists());
    }

    #[test]
    fn clear_all_removes_saved_image_files_and_images_directory() {
        let db = test_db();
        let images_dir = db.app_data_dir.join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        let image_path = images_dir.join("image.png");
        std::fs::write(&image_path, b"png").unwrap();

        db.insert_entry(&image_entry(image_path.clone())).unwrap();
        assert!(image_path.exists());

        db.clear_all().unwrap();

        assert!(!image_path.exists());
        assert!(!images_dir.exists());
    }

    #[test]
    fn old_database_without_icon_column_migrates_and_stores_icon() {
        let dir = std::env::temp_dir().join(format!("xcopy-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // Simulate an old schema: a table created without source_app_icon.
        let db_path = dir.join("xcopy.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE clipboard_history (
                    id TEXT PRIMARY KEY,
                    content_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    source_app TEXT NOT NULL DEFAULT '',
                    preview TEXT NOT NULL DEFAULT '',
                    image_path TEXT,
                    created_at TEXT NOT NULL
                );
                PRAGMA journal_mode=WAL;",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO clipboard_history (id, content_type, content, source_app, preview, image_path, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "old-id",
                    "text",
                    "old content",
                    "Old App",
                    "old content",
                    rusqlite::types::Null,
                    (Utc::now() - ChronoDuration::days(1)).to_rfc3339()
                ],
            )
            .unwrap();
        }

        // Opening with Database::new should trigger the migration that adds the column.
        let db = Database::new(dir).expect("migrated database should open");

        // Old record reads back with source_app_icon == None.
        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "old-id");
        assert_eq!(entries[0].source_app_icon, None);

        // New record carrying an icon path round-trips through the DB.
        let mut entry = text_entry("new content");
        entry.source_app_icon = Some("/some/path/chrome.png".to_string());
        assert!(db.insert_entry_if_changed(&entry).unwrap());

        let entries = db
            .query_entries(&ClipboardFilter {
                query: None,
                content_type: None,
                limit: Some(10),
                offset: Some(0),
            })
            .unwrap();
        assert_eq!(entries.len(), 2);
        let new_entry = entries
            .iter()
            .find(|e| e.content == "new content")
            .unwrap();
        assert_eq!(
            new_entry.source_app_icon,
            Some("/some/path/chrome.png".to_string())
        );
    }
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
        let db_path = app_data_dir.join("xcopy.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                source_app TEXT NOT NULL DEFAULT '',
                source_app_icon TEXT,
                preview TEXT NOT NULL DEFAULT '',
                image_path TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_history(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_content_type ON clipboard_history(content_type);
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;",
        )
        .map_err(|e| e.to_string())?;

        // Migration: older installs created the table without source_app_icon.
        // Add it if missing so existing users keep their history on upgrade.
        let needs_icon_column: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(clipboard_history)")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| e.to_string())?;
            let mut found = false;
            for row in rows {
                if row.map_err(|e| e.to_string())? == "source_app_icon" {
                    found = true;
                    break;
                }
            }
            !found
        };
        if needs_icon_column {
            conn.execute(
                "ALTER TABLE clipboard_history ADD COLUMN source_app_icon TEXT",
                [],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(Database {
            conn: Mutex::new(conn),
            app_data_dir,
            retention_policy: Mutex::new(RetentionPolicy::default()),
        })
    }

    pub fn set_retention_policy(&self, policy: RetentionPolicy) -> Result<(), String> {
        {
            let mut retention_policy = self.retention_policy.lock().map_err(|e| e.to_string())?;
            *retention_policy = policy;
        }
        self.prune_history()
    }

    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<(), String> {
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT OR REPLACE INTO clipboard_history (id, content_type, content, source_app, source_app_icon, preview, image_path, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![entry.id, entry.content_type, entry.content, entry.source_app, entry.source_app_icon, entry.preview, entry.image_path, entry.created_at],
            ).map_err(|e| e.to_string())?;
        }
        self.prune_history()
    }

    pub fn insert_entry_if_changed(&self, entry: &ClipboardEntry) -> Result<bool, String> {
        if let Some(latest) = self.get_last_entry()? {
            if latest.content_type == entry.content_type && latest.content == entry.content {
                return Ok(false);
            }
        }

        self.insert_entry(entry)?;
        Ok(true)
    }

    pub fn query_entries(&self, filter: &ClipboardFilter) -> Result<Vec<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut sql = String::from(
            "SELECT id, content_type, content, source_app, source_app_icon, preview, image_path, created_at FROM clipboard_history WHERE 1=1"
        );
        let mut bind_values: Vec<String> = Vec::new();

        if let Some(ref q) = filter.query {
            if !q.is_empty() {
                sql.push_str(" AND (content LIKE ?1 OR source_app LIKE ?1)");
                bind_values.push(format!("%{}%", q));
            }
        }

        if let Some(ref ct) = filter.content_type {
            if !ct.is_empty() && ct != "all" {
                let idx = bind_values.len() + 1;
                sql.push_str(&format!(" AND content_type = ?{}", idx));
                bind_values.push(ct.clone());
            }
        }

        sql.push_str(" ORDER BY created_at DESC");

        let limit = filter.limit.unwrap_or(100);
        let offset = filter.offset.unwrap_or(0);
        let idx = bind_values.len() + 1;
        sql.push_str(&format!(" LIMIT ?{} OFFSET ?{}", idx, idx + 1));
        bind_values.push(limit.to_string());
        bind_values.push(offset.to_string());

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = bind_values
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();

        let entries = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    source_app: row.get(3)?,
                    source_app_icon: row.get(4)?,
                    preview: row.get(5)?,
                    image_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), String> {
        if let Ok(Some(path)) = self.get_image_path(id) {
            std::fs::remove_file(path).ok();
        }

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), String> {
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute("DELETE FROM clipboard_history", [])
                .map_err(|e| e.to_string())?;
        }

        let images_dir = self.app_data_dir.join("images");
        if images_dir.exists() {
            std::fs::remove_dir_all(images_dir).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn get_last_entry(&self) -> Result<Option<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, content_type, content, source_app, source_app_icon, preview, image_path, created_at
             FROM clipboard_history ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| e.to_string())?;

        let mut entries = stmt
            .query_map([], |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    content: row.get(2)?,
                    source_app: row.get(3)?,
                    source_app_icon: row.get(4)?,
                    preview: row.get(5)?,
                    image_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;

        match entries.next() {
            Some(entry) => Ok(Some(entry.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn get_image_path(&self, id: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT image_path FROM clipboard_history WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    fn prune_history(&self) -> Result<(), String> {
        let policy = *self.retention_policy.lock().map_err(|e| e.to_string())?;
        let cutoff = (Utc::now() - ChronoDuration::days(policy.retention_days)).to_rfc3339();
        let mut image_paths = Vec::new();

        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            collect_image_paths(
                &conn,
                "SELECT image_path FROM clipboard_history WHERE created_at < ?1 AND image_path IS NOT NULL",
                params![cutoff],
                &mut image_paths,
            )?;
            conn.execute(
                "DELETE FROM clipboard_history WHERE created_at < ?1",
                params![cutoff],
            )
            .map_err(|e| e.to_string())?;

            let overflow = overflow_entries(&conn, policy.max_entries)?;
            for (id, image_path) in &overflow {
                if let Some(path) = image_path {
                    image_paths.push(path.clone());
                }
                conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])
                    .map_err(|e| e.to_string())?;
            }
        }

        remove_image_files(&image_paths);
        Ok(())
    }
}

fn overflow_entries(
    conn: &Connection,
    max_entries: usize,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, image_path FROM clipboard_history ORDER BY created_at DESC LIMIT -1 OFFSET ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![max_entries as i64], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn collect_image_paths<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    image_paths: &mut Vec<String>,
) -> Result<(), String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params, |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;

    for row in rows {
        image_paths.push(row.map_err(|e| e.to_string())?);
    }

    Ok(())
}

fn remove_image_files(paths: &[String]) {
    for path in paths {
        if is_inside_images_dir(path) {
            std::fs::remove_file(path).ok();
        }
    }
}

fn is_inside_images_dir(path: &str) -> bool {
    Path::new(path)
        .parent()
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name == "images")
}
