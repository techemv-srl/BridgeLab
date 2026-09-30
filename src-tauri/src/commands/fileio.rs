use serde::Serialize;
use tauri::State;

use super::parser::{parse_message, ParseResult};
use crate::message_store::MessageStore;
use crate::utils::error::BridgeLabError;

/// Start of the error for a path that is a folder; the frontend looks for it.
const FOLDER_ERROR: &str = "not a file but a folder: ";

/// Open a file from disk and parse it.
#[tauri::command]
pub async fn open_file(
    path: String,
    store: State<'_, MessageStore>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<ParseResult, BridgeLabError> {
    // A dropped folder: say so (the read error, or "file not found" for a
    // folder that exists, was all the user got).
    if tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_dir()) {
        return Err(BridgeLabError::FileError(format!("{}{}", FOLDER_ERROR, path)));
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to read {}: {}", path, e)))?;
    // Files in ISO-8859-1 / Windows-1252 (or whatever MSH-18 declares) open
    // as text; the result names the charset, which the tab hands back to
    // save_file so Save and Save As write the file in it again.
    let decoded = crate::parser::hl7::charset::decode_input(&bytes);
    drop(bytes);
    let content = decoded.text;
    let full_text = content.clone();

    // Route by content, not by extension: FHIR resources (JSON/XML) opened
    // from the file dialog, a launch argument or the recent-files list must
    // reach the FHIR parser instead of failing with "does not start with MSH".
    let parsed = if crate::parser::fhir::detect_fhir(&content).is_some() {
        super::parser::parse_fhir_message(content, store, tel)
    } else {
        parse_message(content, Some(path), store, tel)
    };
    // A file that does not parse still opens, unparsed: refusing it left
    // no way to see or fix what is wrong (a stray line before MSH, a
    // truncated header). The frontend says why it is not parsed.
    let mut result = match parsed {
        Ok(r) => r,
        Err(BridgeLabError::ParseError(reason)) => unparsed(&full_text, reason),
        Err(e) => return Err(e),
    };
    result.source_charset = decoded.charset;
    result.charset_warning = decoded.warning;
    result.full_text = Some(full_text);
    Ok(result)
}

/// The result for a file whose text did not parse: no message, no tree.
fn unparsed(text: &str, reason: String) -> ParseResult {
    ParseResult {
        message_id: String::new(),
        message_type: String::new(),
        format: String::new(),
        version: String::new(),
        truncated_text: String::new(),
        tree_roots: Vec::new(),
        truncation_count: 0,
        file_size_bytes: text.len() as u64,
        segment_count: 0,
        source_charset: None,
        full_text: None,
        parse_error: Some(reason),
        charset_warning: None,
    }
}

/// The charset to write `text` in at `path`, first match wins:
/// 1. the one the message declares in MSH-18;
/// 2. `opened_as`, the charset the tab's file was decoded with (Save As of
///    an undeclared legacy file keeps it);
/// 3. the one the file already on disk is in (a tab restored from a
///    session, which does not remember `opened_as`);
/// 4. UTF-8 (`""`).
async fn charset_for_save(text: &str, opened_as: Option<&str>, path: &str) -> String {
    use crate::parser::hl7::charset;
    if let Some(label) = charset::declared_charset(text.as_bytes()).and_then(|c| charset::label_for_hl7(&c)) {
        return if label == "UTF-8" { String::new() } else { label };
    }
    if let Some(label) = opened_as.filter(|l| charset::is_known_label(l)) {
        return label.to_string();
    }
    match tokio::fs::read(path).await {
        Ok(existing) => charset::decode_input(&existing).charset.unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// Save message content to a file.
/// If `content` is provided, it is written directly (use when editor text differs from stored).
/// Otherwise falls back to the MessageStore content using `message_id`.
#[tauri::command]
pub async fn save_file(
    message_id: Option<String>,
    path: String,
    content: Option<String>,
    charset: Option<String>,
    store: State<'_, MessageStore>,
) -> Result<SaveResult, BridgeLabError> {
    let bytes: Vec<u8> = if let Some(c) = content {
        let charset = charset_for_save(&c, charset.as_deref(), &path).await;
        crate::parser::hl7::charset::encode_output(&c, &charset)
    } else if let Some(id) = message_id {
        let msg = store
            .get(&id)
            .ok_or_else(|| BridgeLabError::MessageNotFound(id))?;
        msg.raw.clone()
    } else {
        return Err(BridgeLabError::FileError(
            "Either content or message_id must be provided".into(),
        ));
    };

    let bytes_written = bytes.len() as u64;
    // Atomic: a failed save (disk full, crash) must leave the original file
    // as it was, never truncated and half rewritten.
    let target = path.clone();
    tokio::task::spawn_blocking(move || crate::utils::atomic_write::write_atomic(&target, &bytes))
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to write {}: {}", path, e)))?
        .map_err(|e| BridgeLabError::FileError(format!("Failed to write {}: {}", path, e)))?;

    Ok(SaveResult { path, bytes_written })
}

/// What the tab remembers of its file, to notice a change made by another
/// program: modification time (ms since the epoch) and size.
#[derive(Debug, Serialize, PartialEq)]
pub struct FileStat {
    pub modified_ms: u64,
    pub size: u64,
}

/// The file's modification time and size; `None` when it no longer exists.
/// Other failures (no permission...) are errors: "cannot tell" must not
/// read as "deleted".
#[tauri::command]
pub async fn file_stat(path: String) -> Result<Option<FileStat>, BridgeLabError> {
    stat_of(&path).await
}

async fn stat_of(path: &str) -> Result<Option<FileStat>, BridgeLabError> {
    match tokio::fs::metadata(path).await {
        Ok(m) if m.is_file() => Ok(Some(FileStat {
            modified_ms: m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            size: m.len(),
        })),
        Ok(_) => Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(BridgeLabError::FileError(format!("Failed to read {}: {}", path, e))),
    }
}

/// The file `path` names once symlinks and `.`/`..` are resolved (and, on
/// Windows, case): two tabs on one file reached by different spellings are
/// recognised as one, so their saves cannot overwrite each other. `None`
/// when the file cannot be resolved (it does not exist).
#[tauri::command]
pub async fn canonical_path(path: String) -> Option<String> {
    let real = tokio::fs::canonicalize(&path).await.ok()?;
    Some(without_verbatim_prefix(&real.to_string_lossy()))
}

/// Windows' canonical form is a verbatim path (`\\?\C:\x`, `\\?\UNC\srv\x`):
/// back to the form the rest of the app writes paths in.
pub(crate) fn without_verbatim_prefix(path: &str) -> String {
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", unc)
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct SaveResult {
    pub path: String,
    pub bytes_written: u64,
}

#[cfg(test)]
mod tests {
    use super::{canonical_path, charset_for_save, stat_of, without_verbatim_prefix};

    #[test]
    fn verbatim_prefixes_are_dropped() {
        assert_eq!(without_verbatim_prefix(r"\\?\C:\x\m.hl7"), r"C:\x\m.hl7");
        assert_eq!(without_verbatim_prefix(r"\\?\UNC\srv\share\m.hl7"), r"\\srv\share\m.hl7");
        assert_eq!(without_verbatim_prefix("/tmp/m.hl7"), "/tmp/m.hl7");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn canonical_path_sees_through_symlinked_folders() {
        let dir = std::env::temp_dir().join(format!("bl-canon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("b/m.hl7"), b"MSH").unwrap();
        std::os::unix::fs::symlink(dir.join("b"), dir.join("link")).unwrap();
        let direct = canonical_path(dir.join("b/m.hl7").to_string_lossy().into_owned()).await.unwrap();
        let via_link = canonical_path(dir.join("link/./m.hl7").to_string_lossy().into_owned()).await.unwrap();
        assert_eq!(direct, via_link);
        assert_eq!(canonical_path(dir.join("nope.hl7").to_string_lossy().into_owned()).await, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn stat_reports_size_and_a_missing_file() {
        let dir = std::env::temp_dir().join(format!("bl-stat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("m.hl7");
        let p = path.to_str().unwrap();
        assert_eq!(stat_of(p).await.unwrap(), None);
        std::fs::write(&path, b"MSH|^~\\&|A\r").unwrap();
        let st = stat_of(p).await.unwrap().unwrap();
        assert_eq!(st.size, 11);
        assert!(st.modified_ms > 0);
        assert_eq!(stat_of(dir.to_str().unwrap()).await.unwrap(), None, "a directory is not the file");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn save_charset_follows_msh18_then_the_file_on_disk() {
        let dir = std::env::temp_dir().join(format!("bl-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("m.hl7");
        let p = path.to_str().unwrap();
        let declared = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||ITA|8859/1\rPID|1||1||Müller\r";
        assert_eq!(charset_for_save(declared, None, p).await, "ISO-8859-1");

        let plain = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||1||Müller\r";
        assert_eq!(charset_for_save(plain, None, p).await, "", "no MSH-18 and no file: UTF-8");
        assert_eq!(charset_for_save(plain, Some("windows-1252"), p).await, "windows-1252", "Save As keeps the opened charset");
        assert_eq!(charset_for_save(declared, Some("ISO-8859-2"), p).await, "ISO-8859-1", "MSH-18 wins");
        std::fs::write(&path, b"MSH|^~\\&|A\rPID|1||1||M\xfcller\r").unwrap();
        assert_eq!(charset_for_save(plain, None, p).await, "windows-1252", "keeps the Latin-1 file Latin-1");
        std::fs::write(&path, "MSH|^~\\&|A\rPID|1||1||Müller\r").unwrap();
        assert_eq!(charset_for_save(plain, None, p).await, "");
        // MSH-18 ASCII: Save stays 7-bit, never Windows-1252.
        let ascii = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5|||||USA|ASCII\rPID|1||1||Müller €\r";
        let label = charset_for_save(ascii, Some("windows-1252"), p).await;
        assert_eq!(label, "ASCII");
        let bytes = crate::parser::hl7::charset::encode_output(ascii, &label);
        assert!(bytes.is_ascii(), "{:?}", String::from_utf8_lossy(&bytes));
        std::fs::remove_dir_all(&dir).ok();
    }
}
