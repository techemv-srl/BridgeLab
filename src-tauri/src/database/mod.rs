use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

/// Database manager for BridgeLab local storage.
pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentFile {
    pub path: String,
    pub filename: String,
    pub message_type: String,
    pub version: String,
    pub file_size: u64,
    pub opened_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preference {
    pub key: String,
    pub value: String,
}

impl Database {
    /// Create a new database, initializing tables if needed.
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create db dir: {}", e))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Enable WAL mode for better concurrent performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        Ok(db)
    }

    /// Get the database file path.
    fn db_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Could not determine data directory".to_string())?;
        Ok(data_dir.join("BridgeLab").join("bridgelab.db"))
    }

    /// Run database migrations.
    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS recent_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                message_type TEXT NOT NULL DEFAULT '',
                version TEXT NOT NULL DEFAULT '',
                file_size INTEGER NOT NULL DEFAULT 0,
                opened_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Insert default preferences if not exists
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('theme', 'dark');
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('language', 'en');
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('truncation_threshold', '100');
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('editor_font_size', '13');
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('editor_word_wrap', 'on');
            INSERT OR IGNORE INTO preferences (key, value) VALUES ('tree_width', '350');
            "
        ).map_err(|e| format!("Migration failed: {}", e))?;

        Ok(())
    }

    // --- Recent Files ---

    /// Add or update a recent file entry.
    pub fn add_recent_file(
        &self,
        path: &str,
        filename: &str,
        message_type: &str,
        version: &str,
        file_size: u64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO recent_files (path, filename, message_type, version, file_size, opened_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(path) DO UPDATE SET
                filename = excluded.filename,
                message_type = excluded.message_type,
                version = excluded.version,
                file_size = excluded.file_size,
                opened_at = datetime('now')",
            params![path, filename, message_type, version, file_size as i64],
        ).map_err(|e| format!("Failed to add recent file: {}", e))?;
        Ok(())
    }

    /// Get recent files ordered by most recently opened.
    pub fn get_recent_files(&self, limit: usize) -> Result<Vec<RecentFile>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT path, filename, message_type, version, file_size, opened_at
             FROM recent_files ORDER BY opened_at DESC LIMIT ?1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let files = stmt.query_map(params![limit as i64], |row| {
            Ok(RecentFile {
                path: row.get(0)?,
                filename: row.get(1)?,
                message_type: row.get(2)?,
                version: row.get(3)?,
                file_size: row.get::<_, i64>(4)? as u64,
                opened_at: row.get(5)?,
            })
        }).map_err(|e| format!("Failed to query recent files: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(files)
    }

    /// Remove a recent file entry.
    pub fn remove_recent_file(&self, path: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM recent_files WHERE path = ?1", params![path])
            .map_err(|e| format!("Failed to remove recent file: {}", e))?;
        Ok(())
    }

    /// Clear all recent files.
    pub fn clear_recent_files(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM recent_files", [])
            .map_err(|e| format!("Failed to clear recent files: {}", e))?;
        Ok(())
    }

    // --- Preferences ---

    /// Get a preference value by key.
    pub fn get_preference(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let result = stmt.query_row(params![key], |row| row.get(0)).ok();
        Ok(result)
    }

    /// Set a preference value.
    pub fn set_preference(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        ).map_err(|e| format!("Failed to set preference: {}", e))?;
        Ok(())
    }

    /// Get all preferences.
    pub fn get_all_preferences(&self) -> Result<Vec<Preference>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT key, value FROM preferences ORDER BY key")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let prefs = stmt.query_map([], |row| {
            Ok(Preference {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        }).map_err(|e| format!("Failed to query preferences: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(prefs)
    }
}
