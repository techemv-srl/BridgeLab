use serde::Serialize;
use tauri::State;

use super::parser::{parse_message, ParseResult};
use crate::message_store::MessageStore;
use crate::utils::error::BridgeLabError;

/// Start of the error for a path that is a folder; the frontend looks for it.
const FOLDER_ERROR: &str = "not a file but a folder: ";

/// Files up to this size open straight away. A larger one opens only once
/// the user has agreed (`allow_large`): the app takes far more memory than
/// the file, as the parser indexes every field and the web view lays out
/// every line, and a file of many short messages takes it to several GB.
/// Above the 10 MB messages the documentation and the test plan use.
pub const OPEN_CONFIRM_BYTES: u64 = 16 * 1024 * 1024;

/// No file larger than this opens: one of short messages would take the
/// app past the memory of most machines, and it would be killed with no
/// message (30 MB of them took it past 6 GB). A session tab on such a file
/// was read again at every start.
pub const OPEN_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Starts of the errors for a file that is too large or not a regular file;
/// the frontend looks for them. A size error goes on with
/// `<size> bytes, limit <limit> bytes: <path>`.
const NOT_REGULAR_ERROR: &str = "not a regular file: ";
const TOO_LARGE_ERROR: &str = "too large to open: ";
const CONFIRM_ERROR: &str = "large file, open only if confirmed: ";

/// The file at `path`, read within the open limits: up to
/// `OPEN_CONFIRM_BYTES`, or `OPEN_MAX_BYTES` once the user agreed.
async fn read_for_open(path: &str, allow_large: bool) -> Result<Vec<u8>, BridgeLabError> {
    use crate::utils::bounded_read::{read_regular_file, ReadRefusal};
    let limit = if allow_large { OPEN_MAX_BYTES } else { OPEN_CONFIRM_BYTES };
    let owned = std::path::PathBuf::from(path);
    let read = tokio::task::spawn_blocking(move || read_regular_file(&owned, limit))
        .await
        .map_err(|e| BridgeLabError::FileError(format!("Failed to read {}: {}", path, e)))?;
    read.map_err(|refusal| {
        BridgeLabError::FileError(match refusal {
            ReadRefusal::NotRegular => format!("{}{}", NOT_REGULAR_ERROR, path),
            ReadRefusal::TooLarge { size } if size <= OPEN_MAX_BYTES => {
                format!("{}{} bytes, limit {} bytes: {}", CONFIRM_ERROR, size, OPEN_CONFIRM_BYTES, path)
            }
            ReadRefusal::TooLarge { size } => {
                format!("{}{} bytes, limit {} bytes: {}", TOO_LARGE_ERROR, size, OPEN_MAX_BYTES, path)
            }
            ReadRefusal::Io(e) => format!("Failed to read {}: {}", path, e),
        })
    })
}

/// Open a file from disk and parse it. A file over `OPEN_CONFIRM_BYTES`
/// is refused unless `allow_large` (the user was asked), one over
/// `OPEN_MAX_BYTES` always, and so is anything that is not a regular file:
/// every way of opening a file (dialog, recent files, launch argument,
/// a second launch, session restore) comes here.
#[tauri::command]
pub async fn open_file(
    path: String,
    allow_large: Option<bool>,
    store: State<'_, MessageStore>,
    tel: State<'_, crate::licensing::telemetry::UsageCounters>,
) -> Result<ParseResult, BridgeLabError> {
    // A dropped folder: say so (the read error, or "file not found" for a
    // folder that exists, was all the user got).
    if tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_dir()) {
        return Err(BridgeLabError::FileError(format!("{}{}", FOLDER_ERROR, path)));
    }
    let bytes = read_for_open(&path, allow_large.unwrap_or(false)).await?;
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
    let as_fhir = crate::parser::fhir::detect_fhir(&content).is_some();
    let parsed = if as_fhir {
        super::parser::parse_fhir_message(content, store, tel)
    } else {
        parse_message(content, Some(path), store, tel)
    };
    // A file that does not parse still opens, unparsed: refusing it left
    // no way to see or fix what is wrong (a stray line before MSH, a
    // truncated header). The frontend says why it is not parsed.
    let mut result = match parsed {
        Ok(r) => r,
        Err(BridgeLabError::ParseError(reason)) => {
            let kind = unparsed_kind(&full_text, as_fhir, &reason);
            unparsed(&full_text, reason, kind)
        }
        Err(e) => return Err(e),
    };
    result.source_charset = decoded.charset;
    result.charset_warning = decoded.warning;
    result.full_text = Some(full_text);
    Ok(result)
}

/// Why a file did not parse, as a code the frontend explains in the user's
/// language with a hint (the English `reason` stays as the detail):
/// `empty`, `missing_m` (the line starts with `SH|`), `lowercase_header`
/// (`msh|`), `short_header` (MSH without all its encoding characters),
/// `no_header` (anything else that is not MSH/FHS/BHS), `xml_not_fhir`,
/// and for text routed to the FHIR parser `invalid_json`, `json_not_fhir`
/// or `fhir_invalid`.
pub(crate) fn unparsed_kind(text: &str, as_fhir: bool, reason: &str) -> &'static str {
    if as_fhir {
        return if reason.starts_with("Invalid JSON") {
            "invalid_json"
        } else if reason.starts_with("Not a FHIR resource") {
            "json_not_fhir"
        } else {
            "fhir_invalid"
        };
    }
    // Exactly what the HL7 lexer skips before the header (a byte-order
    // mark, blanks, MLLP framing), so a character it refuses, such as a
    // non-breaking space, counts as something before the header.
    let bytes = text.as_bytes();
    let head = &bytes[crate::parser::hl7::lexer::message_start(bytes)..];
    if head.is_empty() {
        return "empty";
    }
    if head[0] == b'<' {
        return "xml_not_fhir";
    }
    let header = head.get(..3);
    if header.is_some_and(crate::parser::hl7::delimiters::is_header_type) {
        // Long enough, but the delimiters it declares cannot be used (a
        // tab, a non-ASCII character, one character twice).
        if reason.contains(" declares ") {
            return "bad_delimiters";
        }
        return "short_header";
    }
    if header.is_some_and(|h| [b"MSH", b"FHS", b"BHS"].iter().any(|t| h.eq_ignore_ascii_case(*t))) {
        return "lowercase_header";
    }
    if head.starts_with(b"SH") && head.get(2).is_some_and(|c| !c.is_ascii_alphanumeric()) {
        return "missing_m";
    }
    "no_header"
}

/// The result for a file whose text did not parse: no message, no tree.
fn unparsed(text: &str, reason: String, kind: &str) -> ParseResult {
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
        parse_error_kind: Some(kind.to_string()),
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

/// The extensions `allow_save_with_extension` may add: those of the
/// exports the frontend writes itself through the fs plugin (Export JSON
/// and CSV, the XSD dialog's Save as, the batch validation CSV). The
/// test-case pack is written by the backend and needs no grant.
const ADDABLE_EXTENSIONS: [&str; 3] = ["json", "csv", "xsd"];

/// `picked` with `.ext` added to its file name, or why that is refused.
/// `exists`, `is_link` and `in_scope` tell whether a path exists on disk,
/// whether it is a symbolic link, and whether the frontend may already
/// write it (the fs plugin's scope); they are parameters so the rules can
/// be tested without a running app.
///
/// Every condition keeps the grant to the one file the user meant:
/// - `ext` is one of `ADDABLE_EXTENSIONS`, never a name the frontend
///   makes up;
/// - `picked` is an absolute path to a file name, with no `..` part, so
///   `<name>.<ext>` is lexically in the same folder;
/// - `picked` does not exist: the save dialog's answer for a name typed
///   without an extension is a file not created yet, while a file opened
///   or dropped earlier, also in the scope, exists and cannot be used to
///   add files beside it;
/// - `picked` is in the scope, which the save dialog put it in;
/// - `<name>.<ext>` is not a symbolic link: the scope would then admit
///   the file the link points to, which can be anywhere.
fn added_extension_target(
    picked: &std::path::Path,
    ext: &str,
    exists: impl Fn(&std::path::Path) -> bool,
    is_link: impl Fn(&std::path::Path) -> bool,
    in_scope: impl Fn(&std::path::Path) -> bool,
) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    if !ADDABLE_EXTENSIONS.contains(&ext) {
        return Err(format!("The extension .{} is not one BridgeLab adds to a file name", ext));
    }
    // (`components()` already drops a `.` inside the path, which names the
    // same folder anyway.)
    let plain = picked.is_absolute()
        && !picked.as_os_str().to_string_lossy().ends_with(std::path::is_separator)
        && picked.components().all(|c| !matches!(c, Component::ParentDir));
    let Some(name) = picked.file_name().filter(|_| plain) else {
        return Err(format!("{} is not the path of a file", picked.display()));
    };
    if exists(picked) {
        return Err(format!(
            "{} already exists, so .{} cannot be added to its name: type the name with the extension",
            picked.display(),
            ext
        ));
    }
    if !in_scope(picked) {
        return Err(format!("{} was not chosen in the save dialog", picked.display()));
    }
    let mut with_ext = name.to_os_string();
    with_ext.push(".");
    with_ext.push(ext);
    let target = picked.with_file_name(with_ext);
    if is_link(&target) {
        return Err(format!("{} is a link: type the name of the file to write", target.display()));
    }
    Ok(target)
}

/// Whether anything is at `path`, a dangling symbolic link included.
fn path_exists(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn is_symlink(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Lets the frontend write the save dialog's answer with the extension
/// its file type needs, and returns that path.
///
/// The Linux (GTK) save dialog does not add the filter's extension to a
/// name typed without one: it returns `/dir/report`, and the frontend
/// writes `/dir/report.json` (`saveTarget`). The dialog plugin adds only
/// the exact path it returned to the fs scope, so the fs plugin refused
/// the write with "forbidden path" and the export was lost. This adds
/// that one sibling file to the scope, under the conditions of
/// `added_extension_target`, and nothing else: no folder, no pattern, no
/// other name. Like the dialog's own grant it lasts until the app quits.
/// The asset-protocol scope, which the dialog also extends, is left
/// alone: these files are only written.
#[tauri::command]
pub async fn allow_save_with_extension<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    picked: String,
    ext: String,
) -> Result<String, BridgeLabError> {
    use tauri_plugin_fs::FsExt;
    let scope = app
        .try_fs_scope()
        .ok_or_else(|| BridgeLabError::FileError("the file system plugin is not available".into()))?;
    let target = added_extension_target(
        std::path::Path::new(&picked),
        &ext,
        path_exists,
        is_symlink,
        |p| scope.is_allowed(p),
    )
    .map_err(BridgeLabError::FileError)?;
    scope
        .allow_file(&target)
        .map_err(|e| BridgeLabError::FileError(format!("Cannot allow {}: {}", target.display(), e)))?;
    Ok(target.to_string_lossy().into_owned())
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
    use super::{
        added_extension_target, canonical_path, charset_for_save, is_symlink, path_exists, read_for_open, stat_of,
        unparsed_kind, without_verbatim_prefix, CONFIRM_ERROR, NOT_REGULAR_ERROR, OPEN_CONFIRM_BYTES, OPEN_MAX_BYTES,
        TOO_LARGE_ERROR,
    };
    use crate::parser::hl7::lexer::Hl7Lexer;
    use std::path::Path;

    /// What opening a file reads is bounded: a file of the documented 10 MB
    /// opens, a larger one only once the user agreed, one past the hard
    /// limit or a device never. Each used to be read whole, and a device
    /// or a large log of short messages took the app out of memory.
    #[tokio::test]
    async fn opening_a_file_is_bounded() {
        let dir = std::env::temp_dir().join(format!("bl-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sized = |name: &str, len: u64| {
            let p = dir.join(name);
            std::fs::File::create(&p).unwrap().set_len(len).unwrap();
            p.to_string_lossy().into_owned()
        };
        let err = |r: Result<Vec<u8>, crate::utils::error::BridgeLabError>| r.unwrap_err().to_string();

        let ten_mb = sized("ten.hl7", 10 * 1024 * 1024);
        assert_eq!(read_for_open(&ten_mb, false).await.unwrap().len(), 10 * 1024 * 1024);

        let large = sized("large.hl7", OPEN_CONFIRM_BYTES + 1);
        let e = err(read_for_open(&large, false).await);
        // The frontend reads the sizes from this text.
        let sizes = format!("{} bytes, limit {} bytes: {}", OPEN_CONFIRM_BYTES + 1, OPEN_CONFIRM_BYTES, large);
        assert!(e.contains(&format!("{}{}", CONFIRM_ERROR, sizes)), "{e}");
        assert_eq!(read_for_open(&large, true).await.unwrap().len() as u64, OPEN_CONFIRM_BYTES + 1);

        let huge = sized("huge.hl7", OPEN_MAX_BYTES + 1);
        for allow in [false, true] {
            let e = err(read_for_open(&huge, allow).await);
            let sizes = format!("{} bytes, limit {} bytes: {}", OPEN_MAX_BYTES + 1, OPEN_MAX_BYTES, huge);
            assert!(e.contains(&format!("{}{}", TOO_LARGE_ERROR, sizes)), "{e}");
        }
        #[cfg(unix)]
        {
            let e = err(read_for_open("/dev/zero", true).await);
            assert!(e.contains(&format!("{}/dev/zero", NOT_REGULAR_ERROR)), "{e}");
        }
        assert!(err(read_for_open(&dir.join("gone.hl7").to_string_lossy(), false).await).contains("Failed to read"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The save dialog's answer for a name typed without an extension
    /// gets the export's extension, in the same folder: the one sibling
    /// file the frontend may then write. Anything else is refused.
    #[test]
    fn only_the_dialogs_new_file_gets_an_extension() {
        let dir = std::env::temp_dir();
        let picked = dir.join("report");
        let no = |_: &Path| false;
        let grant = |p: &Path, ext: &str| added_extension_target(p, ext, no, no, |q: &Path| q == p);

        assert_eq!(grant(&picked, "json").unwrap(), dir.join("report.json"));
        assert_eq!(grant(&picked, "csv").unwrap(), dir.join("report.csv"));
        assert_eq!(grant(&picked, "xsd").unwrap(), dir.join("report.xsd"));
        assert_eq!(grant(&dir.join("v1.2"), "csv").unwrap(), dir.join("v1.2.csv"));

        // Only the extensions the frontend's exports use, spelled as they do.
        for ext in ["bltests.json", "sh", "desktop", "JSON", ".json", "", "json/../x", "x/json", "json\\x"] {
            assert!(grant(&picked, ext).is_err(), "{ext:?}");
        }
        // Not a plain absolute path to a file name.
        for p in [
            Path::new("report").to_path_buf(),
            dir.join("..").join("report"),
            dir.join("sub").join(".."),
            Path::new(&format!("{}{}", picked.display(), std::path::MAIN_SEPARATOR)).to_path_buf(),
        ] {
            assert!(grant(&p, "json").is_err(), "{}", p.display());
        }
        // Not given by the save dialog (outside the scope), or a file that
        // exists (opened or dropped earlier), or a target that is a link.
        assert!(added_extension_target(&picked, "json", no, no, no).unwrap_err().contains("save dialog"));
        assert!(added_extension_target(&picked, "json", |_| true, no, |_| true).unwrap_err().contains("exists"));
        let target = dir.join("report.json");
        assert!(added_extension_target(&picked, "json", no, |p| p == target.as_path(), |_| true).unwrap_err().contains("link"));
    }

    /// The real file checks the command uses.
    #[cfg(unix)]
    #[test]
    fn the_grant_looks_at_the_disk() {
        let dir = std::env::temp_dir().join(format!("bl-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let in_scope = |_: &Path| true;
        let grant = |name: &str| added_extension_target(&dir.join(name), "json", path_exists, is_symlink, in_scope);

        assert_eq!(grant("new").unwrap(), dir.join("new.json"));
        std::fs::write(dir.join("opened"), b"MSH").unwrap();
        assert!(grant("opened").is_err(), "a file already there");
        std::os::unix::fs::symlink("/nowhere", dir.join("dangling")).unwrap();
        assert!(grant("dangling").is_err(), "a dangling link is there too");
        std::os::unix::fs::symlink("/etc/hostname", dir.join("linked.json")).unwrap();
        assert!(grant("linked").is_err(), "the target is a link");
        // An existing target that is a plain file: replacing it was
        // confirmed by the user in the frontend.
        std::fs::write(dir.join("again.json"), b"{}").unwrap();
        assert_eq!(grant("again").unwrap(), dir.join("again.json"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The kind the HL7 route gives `text`, with the lexer's real error.
    fn hl7_kind(text: &str) -> &'static str {
        let reason = Hl7Lexer::new().parse(text.as_bytes().to_vec()).err().expect("must not parse");
        unparsed_kind(text, false, &reason)
    }

    #[test]
    fn unparsed_files_get_a_kind_the_frontend_can_explain() {
        assert_eq!(hl7_kind("SH|^~\\&|APP|FAC\rPID|1"), "missing_m");
        assert_eq!(hl7_kind("\u{feff}\r\nSH|^~\\&|APP"), "missing_m");
        assert_eq!(hl7_kind("msh|^~\\&|APP|FAC"), "lowercase_header");
        assert_eq!(hl7_kind("Bhs|^~\\&|APP|FAC"), "lowercase_header");
        assert_eq!(hl7_kind("MSH|^~"), "short_header");
        assert_eq!(hl7_kind("PID|1||123"), "no_header");
        assert_eq!(hl7_kind("SHOW ME|1|2|3"), "no_header");
        assert_eq!(hl7_kind("   \r\n "), "empty");
        // A non-breaking space is not skipped by the lexer: it is content
        // before the header, not a short one.
        assert_eq!(hl7_kind("\u{a0}MSH|^~\\&|APP|FAC"), "no_header");
        assert_eq!(hl7_kind("FHS|^"), "short_header");
        assert_eq!(hl7_kind("MSH|^~\rPID|1||PAT1^^^HOSP^MR||Pope^Peter"), "short_header");
        assert_eq!(hl7_kind("MSH\t^~\\&\tAPP\tFAC"), "bad_delimiters");
        assert_eq!(hl7_kind("MSH\u{a6}^~\\&\u{a6}APP"), "bad_delimiters");
        assert_eq!(hl7_kind("MSH|^^\\&|APP|FAC"), "bad_delimiters");
        assert_eq!(hl7_kind("<?xml version=\"1.0\"?><note>hi</note>"), "xml_not_fhir");
        assert_eq!(unparsed_kind("{\"a\":", true, "Invalid JSON: EOF while parsing"), "invalid_json");
        assert_eq!(
            unparsed_kind("{\"a\":1}", true, "Not a FHIR resource: the JSON document has no \"resourceType\""),
            "json_not_fhir"
        );
        assert_eq!(unparsed_kind("<Patient>", true, "XML error"), "fhir_invalid");
    }

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
