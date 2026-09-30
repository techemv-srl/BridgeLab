use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::communication::profiles::{ConnectionProfile, ProfileType, HistoryEntry, HISTORY_KEPT};

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

/// A persisted editor tab (Notepad++-style session restore).
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SessionTab {
    pub tab_order: i64,
    pub label: String,
    pub file_path: Option<String>,
    pub content: String,
    pub is_modified: bool,
    pub is_active: bool,
    pub cursor_line: i64,
    pub cursor_column: i64,
}

/// How many Recent Files entries are kept.
const RECENT_FILES_KEPT: i64 = 30;

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

/// `ALTER TABLE … ADD COLUMN` only when the column is not there yet.
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .map_err(|e| format!("Migration failed: {}", e))?;
    let present = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Migration failed: {}", e))?
        .filter_map(|r| r.ok())
        .any(|name| name == column);
    if !present {
        conn.execute(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, decl), [])
            .map_err(|e| format!("Migration failed: {}", e))?;
    }
    Ok(())
}

/// The database and its -wal/-shm files are readable by their owner only:
/// they hold patient messages. (Windows and macOS already keep the data
/// folder private to the user.)
fn restrict_permissions(db_path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Some(dir) = db_path.parent() {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        for suffix in ["", "-wal", "-shm"] {
            let mut p = db_path.as_os_str().to_owned();
            p.push(suffix);
            let _ = std::fs::set_permissions(std::path::PathBuf::from(p), std::fs::Permissions::from_mode(0o600));
        }
    }
    #[cfg(not(unix))]
    let _ = db_path;
}

/// Replace the credentials in the URLs of every history row (target and
/// preview). Returns how many rows changed.
fn redact_history_credentials(conn: &Connection) -> Result<usize, String> {
    use crate::communication::credentials::redact_urls_in_text;
    let rows: Vec<(String, String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, profile_name, content_preview FROM request_history")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };
    let mut changed = 0;
    for (id, target, preview) in rows {
        let (t, p) = (redact_urls_in_text(&target), redact_urls_in_text(&preview));
        if t != target || p != preview {
            conn.execute(
                "UPDATE request_history SET profile_name = ?1, content_preview = ?2 WHERE id = ?3",
                params![t, p, id],
            )
            .map_err(|e| e.to_string())?;
            changed += 1;
        }
    }
    Ok(changed)
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

        // Enable WAL mode for better concurrent performance. secure_delete
        // overwrites deleted rows with zeros: the database holds message
        // text, request previews and saved headers, and "Clear" must not
        // leave them readable in free pages.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA secure_delete=ON;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        restrict_permissions(&db_path);

        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        db.scrub_once();
        db.redact_history_once();
        Ok(db)
    }

    /// History rows written before URLs were redacted still hold the
    /// user:password (and API keys in query strings) they were sent with,
    /// and the History tab shows them. Redact them once, then rebuild the
    /// file and empty the write-ahead log so the old text is gone from the
    /// disk too.
    fn redact_history_once(&self) {
        const KEY: &str = "history_redacted_v1";
        if matches!(self.get_preference(KEY), Ok(Some(_))) {
            return;
        }
        if let Ok(conn) = self.conn.lock() {
            match redact_history_credentials(&conn) {
                Ok(0) => {}
                Ok(_) => {
                    if conn.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
        let _ = self.set_preference(KEY, "1");
    }

    /// Databases written before secure_delete still hold deleted rows in
    /// their free pages: rebuild once to drop them.
    fn scrub_once(&self) {
        const KEY: &str = "db_scrubbed_v1";
        if matches!(self.get_preference(KEY), Ok(Some(_))) {
            return;
        }
        if let Ok(conn) = self.conn.lock() {
            if conn.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);").is_err() {
                return;
            }
        }
        let _ = self.set_preference(KEY, "1");
    }

    /// Push the write-ahead log into the database and empty it, so rows just
    /// deleted (and zeroed by secure_delete) leave no copy in the -wal file.
    fn scrub(conn: &Connection) {
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
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

            CREATE TABLE IF NOT EXISTS session_tabs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tab_order INTEGER NOT NULL,
                label TEXT NOT NULL,
                file_path TEXT,
                content TEXT NOT NULL,
                is_modified INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 0,
                cursor_line INTEGER NOT NULL DEFAULT 1,
                cursor_column INTEGER NOT NULL DEFAULT 1,
                saved_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            INSERT OR IGNORE INTO preferences (key, value) VALUES ('restore_session', 'true');

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

        // Columns added after a table first shipped. SQLite has no
        // ADD COLUMN IF NOT EXISTS, so each is guarded by the table's own
        // column list; an existing database gets the column once, a new
        // one gets it on top of the CREATE above.
        add_column_if_missing(&conn, "request_history", "ack_code", "TEXT")?;
        add_column_if_missing(&conn, "request_history", "target", "TEXT NOT NULL DEFAULT ''")?;
        add_column_if_missing(&conn, "request_history", "size_bytes", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&conn, "request_history", "request", "TEXT NOT NULL DEFAULT ''")?;
        add_column_if_missing(&conn, "request_history", "response", "TEXT NOT NULL DEFAULT ''")?;

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
        // The menus show a handful; keep a bounded history, not every file
        // ever opened.
        conn.execute(
            "DELETE FROM recent_files WHERE id NOT IN
                (SELECT id FROM recent_files ORDER BY opened_at DESC, id DESC LIMIT ?1)",
            params![RECENT_FILES_KEPT],
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
            ProfileType::Soap => "soap",
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
            let pt = match pt_str.as_str() {
                "http" => ProfileType::Http,
                "soap" => ProfileType::Soap,
                _ => ProfileType::Mllp,
            };
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
        Self::scrub(&conn);
        Ok(())
    }

    // --- Request History ---

    /// Add an entry and keep only the newest [`HISTORY_KEPT`].
    pub fn add_history_entry(&self, entry: &HistoryEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO request_history (id, profile_name, profile_type, direction, content_preview, status, response_time_ms, timestamp, ack_code, target, size_bytes, request, response)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                entry.id, entry.profile_name, entry.profile_type, entry.direction,
                entry.content_preview, entry.status, entry.response_time_ms as i64, entry.timestamp,
                entry.ack_code, entry.target, entry.size_bytes as i64, entry.request, entry.response
            ],
        ).map_err(|e| format!("Failed to add history: {}", e))?;
        let pruned = conn
            .execute(
                "DELETE FROM request_history WHERE id NOT IN \
                 (SELECT id FROM request_history ORDER BY timestamp DESC, rowid DESC LIMIT ?1)",
                params![HISTORY_KEPT as i64],
            )
            .map_err(|e| format!("Failed to prune history: {}", e))?;
        if pruned > 0 {
            // The pruned messages leave no copy in the write-ahead log.
            Self::scrub(&conn);
        }
        Ok(())
    }

    pub fn get_request_history(&self, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, profile_name, profile_type, direction, content_preview, status, response_time_ms, timestamp, ack_code,
                    target, size_bytes, request, response
             FROM request_history ORDER BY timestamp DESC, rowid DESC LIMIT ?1"
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
                ack_code: row.get(8)?,
                target: row.get(9)?,
                size_bytes: row.get::<_, i64>(10)? as u64,
                request: row.get(11)?,
                response: row.get(12)?,
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
        Self::scrub(&conn);
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

    /// Save several test cases in one transaction: an import either lands
    /// completely or not at all.
    pub fn save_test_cases(&self, cases: &[TestCase]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for tc in cases {
            tx.execute(
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
        }
        tx.commit().map_err(|e| format!("Save failed: {}", e))
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

    // --- Session (open tabs) ---

    /// Replace the persisted session with the given list of tabs.
    pub fn save_session(&self, tabs: &[SessionTab]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| format!("Begin tx failed: {}", e))?;

        tx.execute("DELETE FROM session_tabs", [])
            .map_err(|e| format!("Clear session failed: {}", e))?;

        for t in tabs {
            tx.execute(
                "INSERT INTO session_tabs
                 (tab_order, label, file_path, content, is_modified, is_active, cursor_line, cursor_column)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    t.tab_order,
                    t.label,
                    t.file_path,
                    t.content,
                    if t.is_modified { 1_i64 } else { 0 },
                    if t.is_active { 1_i64 } else { 0 },
                    t.cursor_line,
                    t.cursor_column,
                ],
            ).map_err(|e| format!("Insert session tab failed: {}", e))?;
        }

        tx.commit().map_err(|e| format!("Commit failed: {}", e))?;
        Ok(())
    }

    /// Load the persisted session in tab_order.
    pub fn load_session(&self) -> Result<Vec<SessionTab>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT tab_order, label, file_path, content, is_modified, is_active,
                    cursor_line, cursor_column
             FROM session_tabs
             ORDER BY tab_order ASC"
        ).map_err(|e| format!("Prepare failed: {}", e))?;

        let iter = stmt.query_map([], |row| {
            Ok(SessionTab {
                tab_order: row.get::<_, i64>(0)?,
                label: row.get(1)?,
                file_path: row.get(2)?,
                content: row.get(3)?,
                is_modified: row.get::<_, i64>(4)? != 0,
                is_active: row.get::<_, i64>(5)? != 0,
                cursor_line: row.get(6)?,
                cursor_column: row.get(7)?,
            })
        }).map_err(|e| format!("Query failed: {}", e))?;

        Ok(iter.filter_map(|r| r.ok()).collect())
    }

    /// Remove the persisted session.
    pub fn clear_session(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM session_tabs", [])
            .map_err(|e| format!("Clear session failed: {}", e))?;
        Self::scrub(&conn);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_files_are_capped() {
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.migrate().unwrap();
        for i in 0..(RECENT_FILES_KEPT + 5) {
            db.add_recent_file(&format!("/f/{}.hl7", i), "x.hl7", "", "", 0).unwrap();
        }
        let all = db.get_recent_files(1000).unwrap();
        assert_eq!(all.len() as i64, RECENT_FILES_KEPT);
        // The newest survive.
        assert!(all.iter().any(|f| f.path == format!("/f/{}.hl7", RECENT_FILES_KEPT + 4)));
        assert!(!all.iter().any(|f| f.path == "/f/0.hl7"));
    }

    /// The history keeps the newest 100 entries, with their full request
    /// and response.
    #[test]
    fn history_is_pruned_to_the_newest_entries() {
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.migrate().unwrap();
        for i in 0..(HISTORY_KEPT + 20) {
            db.add_history_entry(&HistoryEntry {
                id: format!("e{}", i),
                profile_type: "mllp".into(),
                timestamp: format!("2026-09-30T10:{:02}:{:02}Z", i / 60, i % 60),
                target: "127.0.0.1:2575".into(),
                request: format!("MSH|^~\\&|A|B|C|D|x||ADT^A01|{}|P|2.5\r", i),
                response: "MSH|^~\\&|C|D|A|B|x||ACK|1|P|2.5\rMSA|AA|1\r".into(),
                ..HistoryEntry::default()
            })
            .unwrap();
        }
        let all = db.get_request_history(1000).unwrap();
        assert_eq!(all.len(), HISTORY_KEPT);
        assert_eq!(all[0].id, format!("e{}", HISTORY_KEPT + 19));
        assert!(!all.iter().any(|e| e.id == "e0" || e.id == "e19"));
        assert_eq!(all[0].target, "127.0.0.1:2575");
        assert!(all[0].response.contains("MSA|AA|1"));
        let count: i64 = db.conn.lock().unwrap().query_row("SELECT count(*) FROM request_history", [], |r| r.get(0)).unwrap();
        assert_eq!(count, HISTORY_KEPT as i64);
    }

    /// Passwords and API keys saved in the history by older versions are
    /// redacted once at start.
    #[test]
    fn old_history_credentials_are_redacted() {
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.migrate().unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO request_history (id, profile_name, profile_type, content_preview) VALUES \
                 ('a', 'http://bob:pw123@127.0.0.1:9/creds', 'http', 'GET http://bob:pw123@127.0.0.1:9/creds | '), \
                 ('b', 'Untitled', 'http', 'GET https://h/x?api_key=K9&q=1 | body'), \
                 ('c', 'Untitled', 'mllp', 'MSH|^~\\&|A');",
            )
            .unwrap();
        }
        db.redact_history_once();
        let rows = db.get_request_history(10).unwrap();
        let all: String = rows.iter().map(|r| format!("{} {}\n", r.profile_name, r.content_preview)).collect();
        assert!(!all.contains("pw123") && !all.contains("K9"), "{all}");
        assert!(all.contains("http://***@127.0.0.1:9/creds") && all.contains("api_key=***&q=1"), "{all}");
        assert!(all.contains("MSH|^~\\&|A"));
        assert_eq!(db.get_preference("history_redacted_v1").unwrap().as_deref(), Some("1"));
    }

    /// A history table created before `ack_code` existed gains the column
    /// on migration, and nothing happens when it is already there.
    #[test]
    fn ack_code_column_is_added_once_to_an_older_history_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE request_history (id TEXT PRIMARY KEY, profile_name TEXT NOT NULL, \
             profile_type TEXT NOT NULL, direction TEXT NOT NULL DEFAULT 'send', \
             content_preview TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT '', \
             response_time_ms INTEGER NOT NULL DEFAULT 0, timestamp TEXT NOT NULL DEFAULT '');
             INSERT INTO request_history (id, profile_name, profile_type) VALUES ('old', 'x', 'mllp');",
        )
        .unwrap();
        let columns = |conn: &Connection| -> Vec<String> {
            let mut s = conn.prepare("PRAGMA table_info(request_history)").unwrap();
            s.query_map([], |r| r.get::<_, String>(1)).unwrap().map(|r| r.unwrap()).collect()
        };
        assert!(!columns(&conn).contains(&"ack_code".to_string()));

        add_column_if_missing(&conn, "request_history", "ack_code", "TEXT").unwrap();
        add_column_if_missing(&conn, "request_history", "ack_code", "TEXT").unwrap();
        let cols = columns(&conn);
        assert_eq!(cols.iter().filter(|c| *c == "ack_code").count(), 1);

        // The pre-existing row reads back with no ACK, a new one with its code.
        conn.execute(
            "INSERT INTO request_history (id, profile_name, profile_type, ack_code) VALUES ('new', 'x', 'mllp', 'AE')",
            [],
        )
        .unwrap();
        let old: Option<String> = conn
            .query_row("SELECT ack_code FROM request_history WHERE id = 'old'", [], |r| r.get(0))
            .unwrap();
        let new: Option<String> = conn
            .query_row("SELECT ack_code FROM request_history WHERE id = 'new'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(old, None);
        assert_eq!(new.as_deref(), Some("AE"));
    }
}
