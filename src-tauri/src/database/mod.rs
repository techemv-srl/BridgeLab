use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::communication::profiles::{ConnectionProfile, ProfileType, HistoryEntry};

/// A test case stored for team sharing / regression testing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: String,
    pub content: String,
    pub expected_message_type: String,
    pub expected_validation_result: String,
    pub created_at: String,
    pub updated_at: String,
}

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

            CREATE TABLE IF NOT EXISTS connection_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                profile_type TEXT NOT NULL DEFAULT 'mllp',
                host TEXT NOT NULL DEFAULT 'localhost',
                port INTEGER NOT NULL DEFAULT 2575,
                timeout_secs INTEGER NOT NULL DEFAULT 30,
                url TEXT,
                headers TEXT,
                auto_ack INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS request_history (
                id TEXT PRIMARY KEY,
                profile_name TEXT NOT NULL,
                profile_type TEXT NOT NULL,
                direction TEXT NOT NULL DEFAULT 'send',
                content_preview TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT '',
                response_time_ms INTEGER NOT NULL DEFAULT 0,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS test_cases (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                category TEXT NOT NULL DEFAULT 'general',
                tags TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL,
                expected_message_type TEXT NOT NULL DEFAULT '',
                expected_validation_result TEXT NOT NULL DEFAULT 'valid',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
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

    // --- Connection Profiles ---

    pub fn save_connection_profile(&self, profile: &ConnectionProfile) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let pt = match profile.profile_type {
            ProfileType::Mllp => "mllp",
            ProfileType::Http => "http",
        };
        conn.execute(
            "INSERT INTO connection_profiles (id, name, profile_type, host, port, timeout_secs, url, headers, auto_ack)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, profile_type=excluded.profile_type, host=excluded.host,
                port=excluded.port, timeout_secs=excluded.timeout_secs, url=excluded.url,
                headers=excluded.headers, auto_ack=excluded.auto_ack",
            params![
                profile.id, profile.name, pt, profile.host, profile.port,
                profile.timeout_secs as i64, profile.url, profile.headers,
                profile.auto_ack as i32
            ],
        ).map_err(|e| format!("Failed to save profile: {}", e))?;
        Ok(())
    }

    pub fn get_connection_profiles(&self) -> Result<Vec<ConnectionProfile>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, name, profile_type, host, port, timeout_secs, url, headers, auto_ack
             FROM connection_profiles ORDER BY name"
        ).map_err(|e| format!("Query failed: {}", e))?;

        let profiles = stmt.query_map([], |row| {
            let pt_str: String = row.get(2)?;
            let pt = if pt_str == "http" { ProfileType::Http } else { ProfileType::Mllp };
            Ok(ConnectionProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                profile_type: pt,
                host: row.get(3)?,
                port: row.get::<_, i32>(4)? as u16,
                timeout_secs: row.get::<_, i64>(5)? as u64,
                url: row.get(6)?,
                headers: row.get(7)?,
                auto_ack: row.get::<_, i32>(8)? != 0,
            })
        }).map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
        Ok(profiles)
    }

    pub fn delete_connection_profile(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM connection_profiles WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete failed: {}", e))?;
        Ok(())
    }

    // --- Request History ---

    pub fn add_history_entry(&self, entry: &HistoryEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO request_history (id, profile_name, profile_type, direction, content_preview, status, response_time_ms, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id, entry.profile_name, entry.profile_type, entry.direction,
                entry.content_preview, entry.status, entry.response_time_ms as i64, entry.timestamp
            ],
        ).map_err(|e| format!("Failed to add history: {}", e))?;
        Ok(())
    }

    pub fn get_request_history(&self, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, profile_name, profile_type, direction, content_preview, status, response_time_ms, timestamp
             FROM request_history ORDER BY timestamp DESC LIMIT ?1"
        ).map_err(|e| format!("Query failed: {}", e))?;

        let entries = stmt.query_map(params![limit as i64], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                profile_name: row.get(1)?,
                profile_type: row.get(2)?,
                direction: row.get(3)?,
                content_preview: row.get(4)?,
                status: row.get(5)?,
                response_time_ms: row.get::<_, i64>(6)? as u64,
                timestamp: row.get(7)?,
            })
        }).map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
        Ok(entries)
    }

    pub fn clear_request_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM request_history", [])
            .map_err(|e| format!("Clear failed: {}", e))?;
        Ok(())
    }

    // --- Test Cases ---

    pub fn save_test_case(&self, tc: &TestCase) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO test_cases (id, name, description, category, tags, content,
                expected_message_type, expected_validation_result, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, description=excluded.description,
                category=excluded.category, tags=excluded.tags, content=excluded.content,
                expected_message_type=excluded.expected_message_type,
                expected_validation_result=excluded.expected_validation_result,
                updated_at=datetime('now')",
            params![tc.id, tc.name, tc.description, tc.category, tc.tags, tc.content,
                    tc.expected_message_type, tc.expected_validation_result, tc.created_at],
        ).map_err(|e| format!("Save failed: {}", e))?;
        Ok(())
    }

    pub fn get_test_cases(&self, category_filter: Option<&str>) -> Result<Vec<TestCase>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let query = match category_filter {
            Some(_) => "SELECT id, name, description, category, tags, content, expected_message_type,
                        expected_validation_result, created_at, updated_at
                        FROM test_cases WHERE category = ?1 ORDER BY name",
            None => "SELECT id, name, description, category, tags, content, expected_message_type,
                     expected_validation_result, created_at, updated_at
                     FROM test_cases ORDER BY category, name",
        };

        let mut stmt = conn.prepare(query).map_err(|e| format!("Query prep failed: {}", e))?;

        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<TestCase> {
            Ok(TestCase {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                category: row.get(3)?, tags: row.get(4)?, content: row.get(5)?,
                expected_message_type: row.get(6)?, expected_validation_result: row.get(7)?,
                created_at: row.get(8)?, updated_at: row.get(9)?,
            })
        };

        let cases: Vec<TestCase> = if let Some(cat) = category_filter {
            let iter = stmt.query_map(params![cat], map_row)
                .map_err(|e| format!("Query failed: {}", e))?;
            iter.filter_map(|r| r.ok()).collect()
        } else {
            let iter = stmt.query_map([], map_row)
                .map_err(|e| format!("Query failed: {}", e))?;
            iter.filter_map(|r| r.ok()).collect()
        };

        Ok(cases)
    }

    pub fn delete_test_case(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM test_cases WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete failed: {}", e))?;
        Ok(())
    }
}
