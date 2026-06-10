use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::{ClipboardEntry, ClipboardFilter};

pub struct Database {
    conn: Mutex<Connection>,
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
            preview: content.to_string(),
            image_path: None,
            created_at: chrono::Utc::now().to_rfc3339(),
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

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_entry(&self, entry: &ClipboardEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO clipboard_history (id, content_type, content, source_app, preview, image_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![entry.id, entry.content_type, entry.content, entry.source_app, entry.preview, entry.image_path, entry.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
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
            "SELECT id, content_type, content, source_app, preview, image_path, created_at FROM clipboard_history WHERE 1=1"
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
                    preview: row.get(4)?,
                    image_path: row.get(5)?,
                    created_at: row.get(6)?,
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
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_history", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_last_entry(&self) -> Result<Option<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, content_type, content, source_app, preview, image_path, created_at
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
                    preview: row.get(4)?,
                    image_path: row.get(5)?,
                    created_at: row.get(6)?,
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
}
