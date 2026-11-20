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

/// How long a launch waits for another one that is setting up the same
/// database file.
const SETUP_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Switch the database to WAL. Changing the journal mode needs the file to
/// itself for a moment, and SQLite answers "database is locked" at once
/// when another connection holds it, without waiting for the busy timeout.
/// That happens when several files opened together from a file manager
/// start one BridgeLab each on a new profile: the launch that lost the race
/// ended in a panic (later, in a database in memory) and its file was
/// never opened. Retried until the other launch is done.
fn set_journal_mode(conn: &Connection) -> rusqlite::Result<()> {
    let deadline = std::time::Instant::now() + SETUP_WAIT;
    loop {
        match conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA secure_delete=ON;") {
            Err(e) if is_busy(&e) && std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            other => return other,
        }
    }
}

fn is_busy(e: &rusqlite::Error) -> bool {
    matches!(
        e.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    )
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

/// Replace the credentials in the URLs of every history row (profile
/// name, preview and target). Returns how many rows changed.
fn redact_history_credentials(conn: &Connection) -> Result<usize, String> {
    use crate::communication::credentials::redact_urls_in_text;
    let rows: Vec<(String, String, String, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, profile_name, content_preview, target FROM request_history")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };
    let mut changed = 0;
    for (id, name, preview, target) in rows {
        let (n, p, t) = (redact_urls_in_text(&name), redact_urls_in_text(&preview), redact_urls_in_text(&target));
        if n != name || p != preview || t != target {
            conn.execute(
                "UPDATE request_history SET profile_name = ?1, content_preview = ?2, target = ?3 WHERE id = ?4",
                params![n, p, t, id],
            )
            .map_err(|e| e.to_string())?;
            changed += 1;
        }
    }
    Ok(changed)
}

/// Why the database file could not be used for this run, for the warning
/// the app shows once its window is up.
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseFallback {
    /// The database file; empty when not even its folder is known.
    pub path: String,
    /// The error that stopped it (SQLite's or the file system's words).
    pub reason: String,
    /// The file is fine, but another BridgeLab is using it and this launch
    /// could not hand its files over to it (Linux without a D-Bus session
    /// bus, see `lib.rs`).
    #[serde(default)]
    pub second_instance: bool,
}

/// The database for this run: the file at `path`, or, when it cannot be
/// opened, one in memory together with the reason. A file that is not a
/// database or is damaged, a database locked by another program, a full
/// disk, a folder we may not write to, a file where the folder should
/// be: each of these used to end the app at startup, before any window,
/// without a word (on Windows nothing at all was shown). In memory the
/// editor works as usual; only nothing is kept after the app closes. The
/// file itself is left as it is, never deleted or overwritten, so it can
/// still be repaired or recovered.
pub fn open_or_in_memory(path: Result<PathBuf, String>) -> (Database, Option<DatabaseFallback>) {
    let shown = path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
    match path.and_then(|p| Database::open_at(&p)) {
        Ok(db) => (db, None),
        Err(reason) => (Database::in_memory(), Some(DatabaseFallback { path: shown, reason, second_instance: false })),
    }
}

impl Database {
    /// Create a new database, initializing tables if needed.
    pub fn new() -> Result<Self, String> {
        Self::open_at(&Self::db_path()?)
    }

    /// [`Database::new`] at `db_path`, which is the user's data folder for
    /// the app and a temporary folder for tests.
    pub fn open_at(db_path: &std::path::Path) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create db dir: {}", e))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        // A lock another launch holds for a moment (the migrations' write
        // lock, a rebuild after an upgrade) is waited for, not taken as a
        // database that cannot be opened: that sent the launch to a
        // database in memory, and if it was the one that stayed open,
        // nothing of its session or settings was kept. Set here rather than
        // left to the library's default, which has been 5 s and could
        // change.
        conn.busy_timeout(SETUP_WAIT)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        // Enable WAL mode for better concurrent performance. secure_delete
        // overwrites deleted rows with zeros: the database holds message
        // text, request previews and saved headers, and "Clear" must not
        // leave them readable in free pages.
        set_journal_mode(&conn).map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        restrict_permissions(db_path);

        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        db.scrub_once();
        db.redact_history_once();
        Ok(db)
    }

    /// A database that lives in memory for this run only, with the same
    /// tables and default preferences as a new file.
    pub fn in_memory() -> Self {
        // SQLite fails to open a database in memory, or to create its
        // tables there, only when it cannot allocate memory at all; no
        // window could be shown then either.
        let conn = Connection::open_in_memory().expect("SQLite could not open a database in memory");
        let db = Self { conn: Mutex::new(conn) };
        db.migrate().expect("SQLite could not create the tables of a database in memory");
        db
    }

    /// Go on with a new database in memory instead of this one, whose file
    /// is closed and left as it is: for a second BridgeLab that must not
    /// write over the first one's session and settings (see `lib.rs`).
    pub fn switch_to_memory(&self) {
        let fresh = Database::in_memory();
        // The preferences come along (theme, language, shortcuts), so the
        // window looks and reads like the first; nothing goes back.
        if let Ok(prefs) = self.get_all_preferences() {
            for p in prefs {
                let _ = fresh.set_preference(&p.key, &p.value);
            }
        }
        let fresh = fresh.conn.into_inner().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut conn) = self.conn.lock() {
            *conn = fresh;
        }
    }

    /// History rows written before URLs were redacted still hold the
    /// user:password (and API keys in query strings) they were sent with,
    /// and the History tab shows them. Redact them once, then rebuild the
    /// file and empty the write-ahead log so the old text is gone from the
    /// disk too. The second pass (v2) also covers what the first let
    /// through: a URL that does not parse, whose password was kept in
    /// clear, and tokens in a fragment.
    fn redact_history_once(&self) {
        const KEY: &str = "history_redacted_v2";
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
    pub fn db_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| "Could not determine data directory".to_string())?;
        Ok(data_dir.join("BridgeLab").join("bridgelab.db"))
    }

    /// Run database migrations.
    ///
    /// In one transaction that takes the write lock up front: two launches
    /// creating the same new file each found a column missing and both
    /// added it, and the second failed ("duplicate column name"). IMMEDIATE
    /// waits for the other launch (busy timeout) and then makes the check
    /// and the change one step; the other launch's tables are there by then.
    fn migrate(&self) -> Result<(), String> {
        let mut guard = self.conn.lock().map_err(|e| e.to_string())?;
        let conn = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| format!("Migration failed: {}", e))?;

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

            -- The unsaved tabs of a run the operating system ended (log out,
            -- shut down), offered once at the next start.
            CREATE TABLE IF NOT EXISTS recovery_tabs (
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

        conn.commit().map_err(|e| format!("Migration failed: {}", e))
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
        self.replace_tabs(TabTable::Session, tabs)
    }

    /// Load the persisted session in tab_order.
    pub fn load_session(&self) -> Result<Vec<SessionTab>, String> {
        self.load_tabs(TabTable::Session)
    }

    /// Remove the persisted session.
    pub fn clear_session(&self) -> Result<(), String> {
        self.clear_tabs(TabTable::Session)
    }

    // --- Recovery copy (unsaved tabs when the system ended the app) ---

    /// Keep the unsaved tabs of a run the operating system ended (log
    /// out, shut down) without the window's close request, which would
    /// have asked about them. Written once, at the end of that run, and
    /// offered at the next start; then deleted whatever the user chooses.
    pub fn save_recovery(&self, tabs: &[SessionTab]) -> Result<(), String> {
        self.replace_tabs(TabTable::Recovery, tabs)
    }

    /// The recovery copy, in tab order (empty when there is none).
    pub fn load_recovery(&self) -> Result<Vec<SessionTab>, String> {
        self.load_tabs(TabTable::Recovery)
    }

    /// Delete the recovery copy, leaving no trace of its text in the file.
    pub fn clear_recovery(&self) -> Result<(), String> {
        self.clear_tabs(TabTable::Recovery)
    }

    fn replace_tabs(&self, table: TabTable, tabs: &[SessionTab]) -> Result<(), String> {
        let name = table.name();
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| format!("Begin tx failed: {}", e))?;

        tx.execute(&format!("DELETE FROM {name}"), [])
            .map_err(|e| format!("Clear session failed: {}", e))?;

        for t in tabs {
            tx.execute(
                &format!(
                    "INSERT INTO {name}
                     (tab_order, label, file_path, content, is_modified, is_active, cursor_line, cursor_column)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
                ),
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

    fn load_tabs(&self, table: TabTable) -> Result<Vec<SessionTab>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(&format!(
            "SELECT tab_order, label, file_path, content, is_modified, is_active,
                    cursor_line, cursor_column
             FROM {}
             ORDER BY tab_order ASC",
            table.name()
        )).map_err(|e| format!("Prepare failed: {}", e))?;

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

    fn clear_tabs(&self, table: TabTable) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&format!("DELETE FROM {}", table.name()), [])
            .map_err(|e| format!("Clear session failed: {}", e))?;
        Self::scrub(&conn);
        Ok(())
    }
}

/// The two tables that hold tabs, with the same columns.
#[derive(Clone, Copy)]
enum TabTable {
    Session,
    Recovery,
}

impl TabTable {
    fn name(self) -> &'static str {
        match self {
            TabTable::Session => "session_tabs",
            TabTable::Recovery => "recovery_tabs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bl-db-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The in-memory stand-in behaves like a new database file.
    fn assert_usable(db: &Database) {
        assert_eq!(db.get_preference("theme").unwrap().as_deref(), Some("dark"));
        db.set_preference("k", "v").unwrap();
        assert_eq!(db.get_preference("k").unwrap().as_deref(), Some("v"));
        db.save_session(&[]).unwrap();
        assert!(db.get_test_cases(None).unwrap().is_empty());
    }

    #[test]
    fn a_database_that_opens_is_used_and_kept() {
        let d = temp_dir("ok");
        let path = d.join("BridgeLab").join("bridgelab.db");
        let (db, fallback) = open_or_in_memory(Ok(path.clone()));
        assert!(fallback.is_none(), "{fallback:?}");
        db.set_preference("kept", "yes").unwrap();
        drop(db);
        let (db, fallback) = open_or_in_memory(Ok(path));
        assert!(fallback.is_none());
        assert_eq!(db.get_preference("kept").unwrap().as_deref(), Some("yes"));
        std::fs::remove_dir_all(&d).ok();
    }

    /// Two launches on a new profile: one has the write lock (its
    /// migrations) when the other gets to its own. The second waits and
    /// ends on the file, not on a database in memory that keeps nothing.
    #[test]
    fn a_launch_waits_for_a_sibling_holding_the_write_lock_instead_of_going_to_memory() {
        let d = temp_dir("held-lock");
        let path = d.join("BridgeLab").join("bridgelab.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // The sibling: has made the file and holds the write lock, as the
        // migrations do, for longer than SQLite would wait unasked.
        let held = std::sync::Arc::new(std::sync::Barrier::new(2));
        let sibling = {
            let (path, held) = (path.clone(), held.clone());
            std::thread::spawn(move || {
                let conn = Connection::open(&path).unwrap();
                conn.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE;").unwrap();
                held.wait();
                std::thread::sleep(std::time::Duration::from_millis(600));
                conn.execute_batch("COMMIT;").unwrap();
            })
        };
        held.wait();
        let (db, fallback) = open_or_in_memory(Ok(path.clone()));
        sibling.join().unwrap();
        assert!(fallback.is_none(), "went to a database in memory: {fallback:?}");
        db.set_preference("kept", "yes").unwrap();
        drop(db);
        let db = Database::open_at(&path).unwrap();
        assert_eq!(db.get_preference("kept").unwrap().as_deref(), Some("yes"), "written to the file");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn the_connection_waits_for_locks_as_long_as_the_setup_does() {
        let d = temp_dir("busy-timeout");
        let db = Database::open_at(&d.join("bridgelab.db")).unwrap();
        let ms: i64 = db.conn.lock().unwrap().query_row("PRAGMA busy_timeout", [], |r| r.get(0)).unwrap();
        assert_eq!(ms, SETUP_WAIT.as_millis() as i64);
        std::fs::remove_dir_all(&d).ok();
    }

    /// Several files opened at once from a file manager start one process
    /// each, and every one opens the database before the single-instance
    /// check hands its file to the first. On a new profile they all create
    /// it together: none may fail ("database is locked" while another sets
    /// up the journal, "duplicate column name" when two add the same
    /// column), or its file was never opened.
    #[test]
    fn many_launches_can_create_the_same_new_database_at_once() {
        let d = temp_dir("concurrent");
        for round in 0..12 {
            let path = d.join(format!("p{round}")).join("bridgelab.db");
            let start = std::sync::Arc::new(std::sync::Barrier::new(6));
            let handles: Vec<_> = (0..6)
                .map(|_| {
                    let (path, start) = (path.clone(), start.clone());
                    std::thread::spawn(move || {
                        start.wait();
                        Database::open_at(&path).map(|db| assert_usable(&db))
                    })
                })
                .collect();
            for h in handles {
                if let Err(e) = h.join().unwrap() {
                    panic!("round {round}: {e}");
                }
            }
        }
        std::fs::remove_dir_all(&d).ok();
    }

    /// A second BridgeLab goes on in memory with the same preferences: what
    /// it writes stays out of the file, and the file keeps what the first
    /// one wrote, its session included.
    #[test]
    fn switching_to_memory_leaves_the_file_alone() {
        let d = temp_dir("switch");
        let path = d.join("bridgelab.db");
        let db = Database::open_at(&path).unwrap();
        db.set_preference("language", "it").unwrap();
        let tab = SessionTab {
            tab_order: 0,
            label: "A".into(),
            file_path: None,
            content: "MSH|^~\\&|".into(),
            is_modified: true,
            is_active: true,
            cursor_line: 1,
            cursor_column: 1,
        };
        db.save_session(std::slice::from_ref(&tab)).unwrap();
        db.switch_to_memory();
        assert_eq!(db.get_preference("language").unwrap().as_deref(), Some("it"), "same language as the first window");
        assert!(db.load_session().unwrap().is_empty(), "the first window's session is not shown twice");
        db.set_preference("language", "de").unwrap();
        db.save_session(&[]).unwrap();
        drop(db);
        let db = Database::open_at(&path).unwrap();
        assert_eq!(db.get_preference("language").unwrap().as_deref(), Some("it"));
        assert_eq!(db.load_session().unwrap().len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    /// The recovery copy lives beside the session, apart from it: clearing
    /// the session (restore turned off) leaves it, and discarding it leaves
    /// the session.
    #[test]
    fn the_recovery_copy_is_kept_apart_from_the_session_and_deleted() {
        let d = temp_dir("recovery");
        let path = d.join("bridgelab.db");
        let db = Database::open_at(&path).unwrap();
        assert!(db.load_recovery().unwrap().is_empty());
        let tab = |order: i64, label: &str| SessionTab {
            tab_order: order,
            label: label.into(),
            file_path: None,
            content: format!("MSH|^~\\\\&|{label}"),
            is_modified: true,
            is_active: order == 0,
            cursor_line: 2,
            cursor_column: 3,
        };
        db.save_session(&[tab(0, "session")]).unwrap();
        db.save_recovery(&[tab(1, "second"), tab(0, "first")]).unwrap();
        db.clear_session().unwrap();
        drop(db);
        let db = Database::open_at(&path).unwrap();
        let back = db.load_recovery().unwrap();
        assert_eq!(back.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(), ["first", "second"]);
        assert_eq!(back[0].content, "MSH|^~\\\\&|first");
        assert!(back[0].is_modified && back[0].is_active);
        assert_eq!((back[0].cursor_line, back[0].cursor_column), (2, 3));
        db.save_session(&[tab(0, "session")]).unwrap();
        db.clear_recovery().unwrap();
        assert!(db.load_recovery().unwrap().is_empty());
        assert_eq!(db.load_session().unwrap().len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_new_in_memory_database_is_usable_after_a_switch() {
        let d = temp_dir("switch2");
        let db = Database::open_at(&d.join("bridgelab.db")).unwrap();
        db.switch_to_memory();
        assert_usable(&db);
        std::fs::remove_dir_all(&d).ok();
    }

    /// A file that is not SQLite: the app runs in memory, says why, and
    /// leaves the file exactly as it was.
    #[test]
    fn a_file_that_is_not_a_database_is_left_alone() {
        let d = temp_dir("garbage");
        let path = d.join("bridgelab.db");
        let garbage: Vec<u8> = (0..8192u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        std::fs::write(&path, &garbage).unwrap();
        let (db, fallback) = open_or_in_memory(Ok(path.clone()));
        let fallback = fallback.expect("falls back to memory");
        assert_eq!(fallback.path, path.display().to_string());
        assert!(fallback.reason.contains("not a database"), "{}", fallback.reason);
        assert_usable(&db);
        assert_eq!(std::fs::read(&path).unwrap(), garbage, "the file is not touched");
        std::fs::remove_dir_all(&d).ok();
    }

    /// A real database cut short, as a crash or a full disk can leave it.
    #[test]
    fn a_damaged_database_is_left_alone() {
        let d = temp_dir("damaged");
        let path = d.join("bridgelab.db");
        {
            let db = Database::open_at(&path).unwrap();
            for i in 0..200 {
                db.set_preference(&format!("key{i}"), &"x".repeat(200)).unwrap();
            }
        }
        let whole = std::fs::read(&path).unwrap();
        assert!(whole.len() > 8192, "the test needs a database of several pages");
        std::fs::write(&path, &whole[..3000]).unwrap();
        let (db, fallback) = open_or_in_memory(Ok(path.clone()));
        assert!(fallback.is_some(), "a damaged database falls back to memory");
        assert_usable(&db);
        assert_eq!(std::fs::read(&path).unwrap(), &whole[..3000], "the file is not touched");
        std::fs::remove_dir_all(&d).ok();
    }

    /// A file where the data folder should be, and a data folder that is
    /// not known at all.
    #[test]
    fn a_data_folder_that_cannot_be_used_falls_back_to_memory() {
        let d = temp_dir("nodir");
        std::fs::write(d.join("BridgeLab"), b"a file, not a folder").unwrap();
        let (db, fallback) = open_or_in_memory(Ok(d.join("BridgeLab").join("bridgelab.db")));
        let fallback = fallback.expect("falls back to memory");
        assert!(fallback.reason.starts_with("Failed to create db dir"), "{}", fallback.reason);
        assert_usable(&db);
        assert_eq!(std::fs::read(d.join("BridgeLab")).unwrap(), b"a file, not a folder");

        let (db, fallback) = open_or_in_memory(Err("Could not determine data directory".into()));
        let fallback = fallback.expect("falls back to memory");
        assert_eq!((fallback.path.as_str(), fallback.reason.as_str()), ("", "Could not determine data directory"));
        assert_usable(&db);
        std::fs::remove_dir_all(&d).ok();
    }

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
        assert_eq!(db.get_preference("history_redacted_v2").unwrap().as_deref(), Some("1"));
    }

    /// Rows written after the first pass by a version whose redaction let
    /// a URL that does not parse, and a fragment, through are cleaned by
    /// the second, target included; the first pass's marker does not stop
    /// it.
    #[test]
    fn history_passwords_in_unparseable_urls_and_fragment_tokens_are_redacted() {
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.migrate().unwrap();
        db.set_preference("history_redacted_v1", "1").unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO request_history (id, profile_name, profile_type, content_preview, target) VALUES \
                 ('a', 'http://alice:UNPARSE_PW_11@127.0.0.1:99999/x', 'http', \
                  'GET http://alice:UNPARSE_PW_11@127.0.0.1:99999/x', 'http://alice:UNPARSE_PW_11@127.0.0.1:99999/x'), \
                 ('b', 'Untitled', 'http', 'GET http://h/cb#access_token=FRAG_TOKEN_13&id_token=FRAG_ID_14&state=7', \
                  'http://h/cb#access_token=FRAG_TOKEN_13&id_token=FRAG_ID_14&state=7'), \
                 ('c', 'Untitled', 'http', 'POST http://alice:Pa#ss_A44@h:8080/x', 'http://alice:Pa#ss_A44@h:8080/x');",
            )
            .unwrap();
        }
        db.redact_history_once();
        let rows = db.get_request_history(10).unwrap();
        let all: String =
            rows.iter().map(|r| format!("{} | {} | {}\n", r.profile_name, r.content_preview, r.target)).collect();
        for secret in ["UNPARSE_PW_11", "FRAG_TOKEN_13", "FRAG_ID_14", "Pa#ss", "ss_A44"] {
            assert!(!all.contains(secret), "{secret} in {all}");
        }
        assert!(all.contains("http://***@127.0.0.1:99999/x") && all.contains("state=7"), "{all}");
        assert_eq!(db.get_preference("history_redacted_v2").unwrap().as_deref(), Some("1"));
    }

    /// What a send writes is what the History reads back: a password in a
    /// URL that does not parse, and a fragment token, never reach it.
    #[test]
    fn a_history_entry_round_trips_without_its_credentials() {
        use crate::communication::credentials::redact_credentials;
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.migrate().unwrap();
        for (i, url) in ["http://alice:A45slash/Pw@127.0.0.1/x", "https://h/cb?x=1#access_token=T0K&state=s"].iter().enumerate() {
            // As http_request builds it.
            let target = redact_credentials(url);
            db.add_history_entry(&HistoryEntry {
                id: format!("r{i}"),
                profile_name: target.clone(),
                profile_type: "http".into(),
                content_preview: format!("GET {}", target),
                timestamp: format!("2026-11-19T10:00:0{i}Z"),
                target,
                ..HistoryEntry::default()
            })
            .unwrap();
        }
        let rows = db.get_request_history(10).unwrap();
        let all: String =
            rows.iter().map(|r| format!("{} | {} | {}\n", r.profile_name, r.content_preview, r.target)).collect();
        assert!(!all.contains("A45slash") && !all.contains("Pw@") && !all.contains("T0K"), "{all}");
        assert!(all.contains("http://***@127.0.0.1/x") && all.contains("access_token=***&state=s"), "{all}");
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
