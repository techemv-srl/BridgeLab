use std::sync::Mutex;
use tauri::State;
use crate::database::{Database, DatabaseFallback, RecentFile, Preference, SessionTab};

/// Set when the database file could not be opened and this run uses one in
/// memory (see [`crate::database::open_or_in_memory`]).
pub struct DatabaseFallbackNotice(pub Mutex<Option<DatabaseFallback>>);

/// Why this run has no database file, for the warning the frontend shows
/// at startup: `None` when the file opened. Answers once, so a reload of
/// the window does not repeat the warning.
#[tauri::command]
pub fn take_database_fallback(notice: State<'_, DatabaseFallbackNotice>) -> Option<DatabaseFallback> {
    notice.0.lock().ok().and_then(|mut n| n.take())
}

#[tauri::command]
pub fn get_recent_files(limit: usize, db: State<'_, Database>) -> Result<Vec<RecentFile>, String> {
    db.get_recent_files(limit)
}

#[tauri::command]
pub fn add_recent_file(
    path: String,
    filename: String,
    message_type: String,
    version: String,
    file_size: u64,
    db: State<'_, Database>,
) -> Result<(), String> {
    db.add_recent_file(&path, &filename, &message_type, &version, file_size)
}

#[tauri::command]
pub fn remove_recent_file(path: String, db: State<'_, Database>) -> Result<(), String> {
    db.remove_recent_file(&path)
}

#[tauri::command]
pub fn clear_recent_files(db: State<'_, Database>) -> Result<(), String> {
    db.clear_recent_files()
}

#[tauri::command]
pub fn get_preference(key: String, db: State<'_, Database>) -> Result<Option<String>, String> {
    db.get_preference(&key)
}

#[tauri::command]
pub fn set_preference(key: String, value: String, db: State<'_, Database>) -> Result<(), String> {
    db.set_preference(&key, &value)
}

#[tauri::command]
pub fn get_all_preferences(db: State<'_, Database>) -> Result<Vec<Preference>, String> {
    db.get_all_preferences()
}

#[tauri::command]
pub fn save_session(tabs: Vec<SessionTab>, db: State<'_, Database>) -> Result<(), String> {
    db.save_session(&tabs)
}

#[tauri::command]
pub fn load_session(db: State<'_, Database>) -> Result<Vec<SessionTab>, String> {
    db.load_session()
}

#[tauri::command]
pub fn clear_session(db: State<'_, Database>) -> Result<(), String> {
    db.clear_session()
}

/// The tabs as the window last handed them over (a few tenths of a second
/// after an edit, and at once when it loses focus or is hidden), held in
/// memory only, for the moment the operating system ends the app without
/// the window's close request: log out, shut down, the standard macOS
/// Quit, Windows' end of session. Then the window cannot be asked any more
/// (on Windows and macOS that moment comes on the thread the window would
/// answer through), and this is what gets written, once (see `lib.rs`):
///
/// - with session restore on, the whole tab set, as the session, so the
///   next start restores the latest edits like any session and offers
///   nothing twice;
/// - with restore off, only the unsaved tabs, as the recovery copy offered
///   at the next start.
///
/// The window empties it when it closes the normal way, having asked about
/// the unsaved tabs itself (or saved the session).
#[derive(Default)]
pub struct ExitSnapshot(Mutex<Option<Snapshot>>);

pub struct Snapshot {
    tabs: Vec<SessionTab>,
    /// The whole session (restore on), not just the unsaved tabs.
    session: bool,
}

impl ExitSnapshot {
    /// An empty list of unsaved tabs drops the snapshot; an empty session
    /// is kept, since closing every tab is a session too.
    pub fn set(&self, tabs: Vec<SessionTab>, session: bool) {
        if let Ok(mut s) = self.0.lock() {
            *s = (session || !tabs.is_empty()).then_some(Snapshot { tabs, session });
        }
    }

    /// The snapshot, at most once: the end of the app can be reached by
    /// two paths (the exit event and the signal watchdog).
    fn take(&self) -> Option<Snapshot> {
        self.0.lock().ok().and_then(|mut s| s.take())
    }

    /// Write the snapshot: as the session, or as the recovery copy.
    pub fn save_to(&self, db: &Database) {
        let written = match self.take() {
            Some(Snapshot { tabs, session: true }) => db.save_session(&tabs),
            Some(Snapshot { tabs, session: false }) => db.save_recovery(&tabs),
            None => return,
        };
        if let Err(e) = written {
            eprintln!("BridgeLab: the open tabs could not be kept at exit: {e}");
        }
    }
}

#[tauri::command]
pub fn set_exit_snapshot(tabs: Vec<SessionTab>, session: bool, snapshot: State<'_, ExitSnapshot>) {
    snapshot.set(tabs, session);
}

#[tauri::command]
pub fn load_recovery_tabs(db: State<'_, Database>) -> Result<Vec<SessionTab>, String> {
    db.load_recovery()
}

#[tauri::command]
pub fn clear_recovery_tabs(db: State<'_, Database>) -> Result<(), String> {
    db.clear_recovery()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(label: &str) -> SessionTab {
        SessionTab {
            tab_order: 0,
            label: label.into(),
            file_path: None,
            content: "MSH|^~\\&|".into(),
            is_modified: true,
            is_active: true,
            cursor_line: 1,
            cursor_column: 1,
        }
    }

    #[test]
    fn the_snapshot_is_written_once_and_an_empty_one_writes_nothing() {
        let db = Database::in_memory();
        let snapshot = ExitSnapshot::default();
        snapshot.set(vec![tab("a")], false);
        snapshot.save_to(&db);
        assert_eq!(db.load_recovery().unwrap().len(), 1);
        db.clear_recovery().unwrap();
        snapshot.save_to(&db);
        assert!(db.load_recovery().unwrap().is_empty(), "taken by the first write");

        // Closed the normal way: the window emptied it.
        snapshot.set(vec![tab("b")], false);
        snapshot.set(Vec::new(), false);
        snapshot.save_to(&db);
        assert!(db.load_recovery().unwrap().is_empty());
    }

    /// With restore on the latest tab set replaces the session and no
    /// recovery copy is made: the next start restores the tabs once.
    #[test]
    fn with_restore_on_the_snapshot_becomes_the_session_not_a_recovery_copy() {
        let db = Database::in_memory();
        db.save_session(&[tab("older autosave")]).unwrap();
        let snapshot = ExitSnapshot::default();
        snapshot.set(vec![tab("latest edits")], true);
        snapshot.save_to(&db);
        assert_eq!(db.load_session().unwrap()[0].label, "latest edits");
        assert!(db.load_recovery().unwrap().is_empty(), "nothing offered twice");

        // Every tab closed: the session is emptied too.
        snapshot.set(Vec::new(), true);
        snapshot.save_to(&db);
        assert!(db.load_session().unwrap().is_empty());
    }

    /// Turning restore on or off replaces the snapshot of the other kind.
    #[test]
    fn the_latest_kind_wins() {
        let db = Database::in_memory();
        let snapshot = ExitSnapshot::default();
        snapshot.set(vec![tab("unsaved")], false);
        snapshot.set(vec![tab("unsaved"), tab("saved")], true);
        snapshot.save_to(&db);
        assert!(db.load_recovery().unwrap().is_empty());
        assert_eq!(db.load_session().unwrap().len(), 2);
    }
}
