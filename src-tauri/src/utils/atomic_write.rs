//! Replace a file without ever leaving it half-written.
//!
//! `fs::write` truncates the target first and then writes: if the disk
//! fills or the process dies in between, the original is gone and a
//! partial file is left behind (a failed Save could destroy the message it
//! was saving). Here the new content goes to a temporary file in
//! the same directory, is flushed to disk, and only then renamed over the
//! target in one step. On any error the target is untouched.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The end of a temporary file's name: `.<file name>.<16 hex digits>` and this.
const TEMP_SUFFIX: &str = ".bridgelab-tmp";

/// Create a fresh temporary file next to `target`: an unpredictable name,
/// opened with create_new so an existing file or a planted symlink is
/// never followed or truncated. Retries on a name collision.
///
/// `private`: readable and writable by us only (Unix), for the save of an
/// existing file. Created with the default mode (0644 under the usual
/// umask), it held the whole new text, readable by every local user who
/// could list the folder, until its mode was set to the target's just
/// before the rename, and for good if the save was killed in between. A
/// new file gets the default mode, as it will have once saved.
fn create_temp(dir: &Path, name: &str, private: bool) -> io::Result<(PathBuf, File)> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    for _ in 0..32 {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let salt = (nanos as u64) ^ (std::process::id() as u64).rotate_left(32) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let tmp = dir.join(format!(".{}.{:016x}{}", name, salt, TEMP_SUFFIX));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        if private {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(not(unix))]
        let _ = private;
        match options.open(&tmp) {
            Ok(f) => return Ok((tmp, f)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "could not create a unique temporary file"))
}

/// How old a temporary file left in the folder must be before a save
/// removes it: long past any save still running in another BridgeLab
/// window or in the CLI.
const STALE_TEMP_AGE: Duration = Duration::from_secs(10 * 60);

/// The name of the file a temporary file named `temp_name` was saving,
/// when `temp_name` is exactly one of ours: `.<name>.<16 hex>.bridgelab-tmp`.
fn temp_owner_name(temp_name: &str) -> Option<&str> {
    let rest = temp_name.strip_prefix('.')?.strip_suffix(TEMP_SUFFIX)?;
    let (name, salt) = rest.rsplit_once('.')?;
    let ours = !name.is_empty() && salt.len() == 16 && salt.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    ours.then_some(name)
}

/// Remove the temporary files that saves into `dir` left behind when the
/// app was killed or the machine went down before the rename: each holds
/// a full copy of a message, and before 2.1 it could be readable by
/// everyone. Only regular files with exactly our name pattern, made by
/// us (Unix), at least [`STALE_TEMP_AGE`] old, and whose file is still
/// there (the save never replaced it) go. Best effort: errors are ignored.
fn remove_stale_temps(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(owner) = file_name.to_str().and_then(temp_owner_name) else { continue };
        // Not following a symbolic link: only our own regular files.
        let Ok(meta) = entry.metadata() else { continue };
        let old = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).is_some_and(|age| age >= STALE_TEMP_AGE);
        if meta.is_file() && old && made_by_us(&meta) && dir.join(owner).is_file() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(unix)]
fn made_by_us(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: geteuid has no preconditions and cannot fail.
    meta.uid() == unsafe { libc::geteuid() }
}

#[cfg(not(unix))]
fn made_by_us(_meta: &fs::Metadata) -> bool {
    true
}

#[cfg(test)]
type TempHook = Box<dyn Fn(&Path)>;

// Lets a test look at the temporary file once it holds the new text,
// before its mode is set to the target's.
#[cfg(test)]
thread_local! {
    static TEMP_WRITTEN: std::cell::RefCell<Option<TempHook>> = const { std::cell::RefCell::new(None) };
}

/// Write `bytes` to `path` atomically. A symlink is followed (the file it
/// points to is replaced, the link stays; a link to a missing file creates
/// that file), and an existing file keeps its permissions and, where the
/// OS lets us, its owner and group.
///
/// Replacing the file is what makes the save atomic, but a new file is not
/// the same file in every respect. Where the difference would lose
/// something, the file is rewritten in place instead, which keeps it but
/// is not atomic (a crash mid-write can leave it partly written):
/// - a hard-linked file: a new file would leave the other names holding
///   the old text;
/// - a file whose owner or group we cannot give the new file (someone
///   else's file we may write to through the group): replacing it would
///   hand it to us and could lock its owner out;
/// - a writable file in a folder we cannot create files in, which the
///   pre-1.9 plain write could save and the temporary file cannot;
/// - a file whose ACL or extended attributes cannot be given to the new
///   file (see `copy_extended_attributes`).
///
/// The new file gets the ACL and extended attributes of the one it
/// replaces: on Linux by copying them, on macOS with copyfile(3), on
/// Windows through ReplaceFileW (see `replace`).
pub fn write_atomic(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    let path = path.as_ref();
    let target = resolve_target(path);
    let existing = fs::metadata(&target).ok().filter(|m| m.is_file());
    if let Some(meta) = &existing {
        refuse_read_only(&target, meta)?;
    }
    if existing.as_ref().is_some_and(|meta| is_hard_linked(&target, meta)) {
        return write_in_place(&target, bytes);
    }
    let dir = target.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    remove_stale_temps(dir);
    let (tmp, mut f) = match create_temp(dir, &name, existing.is_some()) {
        Ok(created) => created,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && existing.is_some() => {
            return write_in_place(&target, bytes);
        }
        Err(e) => return Err(e),
    };

    let result = (|| {
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        #[cfg(test)]
        TEMP_WRITTEN.with(|hook| {
            if let Some(hook) = &*hook.borrow() {
                hook(&tmp);
            }
        });
        if let Some(meta) = &existing {
            if !copy_owner(meta, &tmp) {
                return Ok(false);
            }
            let _ = fs::set_permissions(&tmp, meta.permissions());
            // After the mode: on Linux, setting the ACL sets the group
            // bits from its mask again.
            if !copy_extended_attributes(&target, &tmp) {
                return Ok(false);
            }
        }
        replace(&tmp, &target, existing.is_some()).map(|_| true)
    })();
    match result {
        Ok(true) => Ok(()),
        Ok(false) => {
            let _ = fs::remove_file(&tmp);
            write_in_place(&target, bytes)
        }
        Err(e) => {
            // A failed replace on Windows can leave the original gone and
            // the new text only in the temporary file: that copy stays.
            if existing.is_some() && !target.exists() && tmp.exists() {
                return Err(io::Error::new(e.kind(), format!("{}; the saved text is in {}", e, tmp.display())));
            }
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Put `tmp` in the place of `target`. An existing file on Windows is
/// replaced with ReplaceFileW, which gives the new file the old one's ACL,
/// attributes, alternate data streams and creation time; a rename keeps
/// none of them. Where ReplaceFileW fails (a file system or share that
/// does not support it), the rename is used as before.
#[cfg(windows)]
fn replace(tmp: &Path, target: &Path, existed: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_IGNORE_MERGE_ERRORS};
    if existed {
        let wide = |p: &Path| p.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<u16>>();
        let (replaced, replacement) = (wide(target), wide(tmp));
        // SAFETY: two NUL-terminated paths that outlive the call; no backup
        // file, and the two reserved pointers are null as required.
        let ok = unsafe {
            ReplaceFileW(
                replaced.as_ptr(),
                replacement.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_IGNORE_MERGE_ERRORS,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if ok != 0 {
            return Ok(());
        }
    }
    // MoveFileEx with MOVEFILE_REPLACE_EXISTING. After a failed
    // ReplaceFileW the original is still there, or already gone
    // (ERROR_UNABLE_TO_MOVE_REPLACEMENT): either way this puts the new
    // file in its place.
    fs::rename(tmp, target)
}

/// Put `tmp` in the place of `target`, in one step.
#[cfg(not(windows))]
fn replace(tmp: &Path, target: &Path, _existed: bool) -> io::Result<()> {
    fs::rename(tmp, target)
}

/// Give `tmp` the extended attributes of `target`, the file it replaces.
/// On Linux they hold its ACL (`system.posix_acl_access`), its SELinux
/// label and any `user.*` attribute. A new file has none of them, while
/// the mode it is given carries the ACL mask in its group bits: without
/// the ACL, a file its owning group could only read (or not open at all)
/// became one that group could write, and the users the ACL named lost
/// access. False when one cannot be copied: the caller then writes in
/// place, which keeps them all.
#[cfg(target_os = "linux")]
fn copy_extended_attributes(target: &Path, tmp: &Path) -> bool {
    const ACL: &str = "system.posix_acl_access";
    let names: Vec<std::ffi::OsString> = match xattr::list(target) {
        Ok(names) => names.collect(),
        // A file system without extended attributes: nothing to carry over.
        Err(e) if e.raw_os_error() == Some(libc::ENOTSUP) => return true,
        Err(_) => return false,
    };
    for name in &names {
        let value = match xattr::get(target, name) {
            Ok(Some(value)) => value,
            Ok(None) => continue,
            Err(_) => return false,
        };
        // Already there with that value: the SELinux label every new file
        // in the folder gets, or an ACL inherited from the folder.
        if matches!(xattr::get(tmp, name), Ok(Some(v)) if v == value) {
            continue;
        }
        if xattr::set(tmp, name, &value).is_err() {
            return false;
        }
    }
    // An ACL the new file inherited from the folder's default ACL, which
    // the file it replaces did not have, would give access to more users.
    if !names.iter().any(|n| n.as_os_str() == ACL) && matches!(xattr::get(tmp, ACL), Ok(Some(_))) {
        return xattr::remove(tmp, ACL).is_ok();
    }
    true
}

/// Give `tmp` the ACL and extended attributes (Finder tags and the like)
/// of `target`, the file it replaces, with copyfile(3). False when they
/// cannot be copied: the caller then writes in place, which keeps them.
#[cfg(target_os = "macos")]
fn copy_extended_attributes(target: &Path, tmp: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let (Ok(from), Ok(to)) = (CString::new(target.as_os_str().as_bytes()), CString::new(tmp.as_os_str().as_bytes())) else {
        return false;
    };
    // SAFETY: two NUL-terminated paths that outlive the call, and no state.
    unsafe { libc::copyfile(from.as_ptr(), to.as_ptr(), std::ptr::null_mut(), libc::COPYFILE_ACL | libc::COPYFILE_XATTR) == 0 }
}

/// Nothing to copy here: on Windows `replace` carries them over.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn copy_extended_attributes(_target: &Path, _tmp: &Path) -> bool {
    true
}

/// A write-protected file is not replaced. Renaming over a file only needs
/// a writable folder, so without this check a read-only file (`chmod 444`,
/// or one we may not write to) was silently replaced on Linux and macOS,
/// while Windows refuses. Read-only means no write permission bit at all
/// (checked for root too, as on Windows), or a file we cannot open for
/// writing.
fn refuse_read_only(target: &Path, meta: &fs::Metadata) -> io::Result<()> {
    let denied = meta.permissions().readonly()
        || matches!(OpenOptions::new().write(true).open(target), Err(e) if e.kind() == io::ErrorKind::PermissionDenied);
    if denied {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the file is read-only"));
    }
    Ok(())
}

/// The file a save of `path` must write: `path` with every symlink
/// resolved. `canonicalize` fails on a link to a file that does not exist
/// yet; its target is then followed by hand, so the save creates that file
/// instead of replacing the link with a regular file.
fn resolve_target(path: &Path) -> PathBuf {
    if let Ok(real) = fs::canonicalize(path) {
        return real;
    }
    let mut current = path.to_path_buf();
    for _ in 0..40 {
        match fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => match fs::read_link(&current) {
                Ok(next) => {
                    current = if next.is_absolute() {
                        next
                    } else {
                        current.parent().map(|p| p.join(&next)).unwrap_or(next)
                    };
                }
                Err(_) => break,
            },
            _ => break,
        }
    }
    current
}

#[cfg(unix)]
fn is_hard_linked(_target: &Path, meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() > 1
}

/// The link count is not in std's stable metadata on Windows: it is asked
/// of the file itself. A hard-linked file used to be replaced on save, so
/// the other name kept the old text.
#[cfg(windows)]
fn is_hard_linked(target: &Path, _meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    // Opened for no access at all: enough to read its information, and
    // allowed while another program has the file open.
    let Ok(file) = OpenOptions::new().access_mode(0).open(target) else { return false };
    // SAFETY: an all-zero BY_HANDLE_FILE_INFORMATION is a valid value of
    // this plain C struct, and the handle stays open for the call.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    ok != 0 && info.nNumberOfLinks > 1
}

#[cfg(not(any(unix, windows)))]
fn is_hard_linked(_target: &Path, _meta: &fs::Metadata) -> bool {
    false
}

/// Give `tmp` the owner and group of the file it replaces. False when that
/// is not possible (the caller then writes in place).
#[cfg(unix)]
fn copy_owner(meta: &fs::Metadata, tmp: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Ok(now) = fs::metadata(tmp) else { return false };
    if now.uid() == meta.uid() && now.gid() == meta.gid() {
        return true;
    }
    if std::os::unix::fs::chown(tmp, Some(meta.uid()), Some(meta.gid())).is_ok() {
        return true;
    }
    // Not root: the owner can only be ours, the group any we belong to.
    now.uid() == meta.uid() && std::os::unix::fs::chown(tmp, None, Some(meta.gid())).is_ok()
}

#[cfg(not(unix))]
fn copy_owner(_meta: &fs::Metadata, _tmp: &Path) -> bool {
    true
}

/// Overwrite the file itself (truncate and write): keeps its inode, owner,
/// links and attributes, but is not atomic.
fn write_in_place(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = OpenOptions::new().write(true).truncate(true).open(target)?;
    f.write_all(bytes)?;
    f.sync_all()
}

#[cfg(test)]
mod tests {
    use super::write_atomic;
    use std::fs;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bl-atomic-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn replaces_and_creates() {
        let d = dir("basic");
        let p = d.join("m.hl7");
        write_atomic(&p, b"one").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"one");
        write_atomic(&p, b"two, longer").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"two, longer");
        // No temporary file left behind.
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1);
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_failed_write_leaves_the_original_intact() {
        let d = dir("fail");
        let p = d.join("m.hl7");
        fs::write(&p, b"original").unwrap();
        // Make the directory read-only: the temporary file cannot be created.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&d, fs::Permissions::from_mode(0o555)).unwrap();
            let r = write_atomic(&p, b"new content");
            fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
            // root ignores directory permissions; only assert when it failed.
            if r.is_err() {
                assert_eq!(fs::read(&p).unwrap(), b"original");
            }
        }
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn concurrent_saves_never_share_a_temporary_file() {
        let d = dir("concurrent");
        let p = d.join("m.hl7");
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let p = p.clone();
                std::thread::spawn(move || write_atomic(&p, format!("version {i}").repeat(1000).as_bytes()).unwrap())
            })
            .collect();
        for h in handles { h.join().unwrap(); }
        let text = String::from_utf8(fs::read(&p).unwrap()).unwrap();
        // One whole version, never an interleaving of two.
        let first = &text[..9];
        assert_eq!(text, first.repeat(1000));
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1, "no temporary file left");
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(unix)]
    #[test]
    fn follows_a_symlink_and_keeps_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("link");
        let real = d.join("real.hl7");
        let link = d.join("link.hl7");
        fs::write(&real, b"old").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write_atomic(&link, b"new").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&real).unwrap(), b"new");
        assert_eq!(fs::metadata(&real).unwrap().permissions().mode() & 0o777, 0o640);
        fs::remove_dir_all(&d).ok();
    }

    // NTFS has hard links too (`fsutil hardlink create`), and a save there
    // must reach the other name as it does on Unix.
    #[test]
    fn a_hard_linked_file_is_written_through() {
        let d = dir("hardlink");
        let a = d.join("a.hl7");
        let b = d.join("b.hl7");
        fs::write(&a, b"old").unwrap();
        fs::hard_link(&a, &b).unwrap();
        write_atomic(&a, b"new").unwrap();
        assert_eq!(fs::read(&b).unwrap(), b"new", "the other name sees the save");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 2, "no temporary file left");
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_creates_its_target() {
        let d = dir("dangling");
        let link = d.join("dangling.hl7");
        std::os::unix::fs::symlink("missing.hl7", &link).unwrap();
        write_atomic(&link, b"new").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink(), "the link stays a link");
        assert_eq!(fs::read(d.join("missing.hl7")).unwrap(), b"new");
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(unix)]
    #[test]
    fn owner_and_group_are_kept() {
        use std::os::unix::fs::MetadataExt;
        let d = dir("owner");
        let p = d.join("m.hl7");
        fs::write(&p, b"old").unwrap();
        let before = fs::metadata(&p).unwrap();
        // As root, give the file to another owner: the save must keep it.
        let other = if before.uid() == 0 { std::os::unix::fs::chown(&p, Some(1), Some(1)).is_ok() } else { false };
        write_atomic(&p, b"new").unwrap();
        let after = fs::metadata(&p).unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        if other {
            assert_eq!((after.uid(), after.gid()), (1, 1));
        } else {
            assert_eq!((after.uid(), after.gid()), (before.uid(), before.gid()));
        }
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_read_only_file_is_refused_and_left_as_it_is() {
        let d = dir("readonly");
        let p = d.join("golden.json");
        fs::write(&p, b"golden").unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&p, perms).unwrap();
        let err = write_atomic(&p, b"new").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("read-only"), "{err}");
        assert_eq!(fs::read(&p).unwrap(), b"golden");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1, "no temporary file left");
        let mut perms = fs::metadata(&p).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&p, perms).unwrap();
        fs::remove_dir_all(&d).ok();
    }

    /// While the temporary file holds the new text it is never readable by
    /// more users than the file it replaces: it used to be created with
    /// the default mode and given the target's only before the rename.
    #[cfg(unix)]
    #[test]
    fn the_temporary_file_is_never_more_open_than_the_target() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("tempmode");
        let p = d.join("secret.hl7");
        for target_mode in [0o600, 0o640, 0o644] {
            fs::write(&p, b"old").unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(target_mode)).unwrap();
            let seen = std::rc::Rc::new(std::cell::Cell::new(None));
            let record = seen.clone();
            super::TEMP_WRITTEN.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |tmp: &std::path::Path| {
                    assert_eq!(fs::read(tmp).unwrap(), b"patient");
                    record.set(Some(fs::metadata(tmp).unwrap().permissions().mode() & 0o7777));
                }));
            });
            let saved = write_atomic(&p, b"patient");
            super::TEMP_WRITTEN.with(|hook| *hook.borrow_mut() = None);
            saved.unwrap();
            let temp_mode = seen.get().expect("the hook saw the temporary file");
            assert_eq!(temp_mode & !target_mode, 0, "temporary {temp_mode:o}, target {target_mode:o}");
            assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o7777, target_mode);
            assert_eq!(fs::read(&p).unwrap(), b"patient");
        }
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn our_temporary_file_names_are_recognised_exactly() {
        use super::temp_owner_name;
        assert_eq!(temp_owner_name(".m.hl7.0123456789abcdef.bridgelab-tmp"), Some("m.hl7"));
        assert_eq!(temp_owner_name("..hidden.0123456789abcdef.bridgelab-tmp"), Some(".hidden"));
        for other in [
            "m.hl7.0123456789abcdef.bridgelab-tmp",
            ".m.hl7.0123456789ABCDEF.bridgelab-tmp",
            ".m.hl7.0123456789abcde.bridgelab-tmp",
            ".m.hl7.0123456789abcdef.bridgelab-tmp.bak",
            "..0123456789abcdef.bridgelab-tmp",
            ".0123456789abcdef.bridgelab-tmp",
        ] {
            assert_eq!(temp_owner_name(other), None, "{other}");
        }
    }

    /// The temporary file of a save that was killed before the rename is
    /// removed by a later save into the folder, but not one that may still
    /// be in use, nor one that may be the only copy of its text.
    #[test]
    fn temporary_files_left_by_killed_saves_are_removed() {
        use std::time::{Duration, SystemTime};
        let d = dir("stale");
        let p = d.join("m.hl7");
        fs::write(&p, b"old").unwrap();
        let hour_ago = SystemTime::now() - Duration::from_secs(3600);
        let leave = |name: &str, aged: bool| {
            let f = fs::File::create(d.join(name)).unwrap();
            if aged {
                f.set_modified(hour_ago).unwrap();
            }
        };
        leave(".m.hl7.00000000000000aa.bridgelab-tmp", true); // left by a killed save
        leave(".m.hl7.00000000000000bb.bridgelab-tmp", false); // a save running now
        leave(".gone.hl7.00000000000000cc.bridgelab-tmp", true); // its file is gone
        leave(".m.hl7.tmp", true); // not ours
        write_atomic(&p, b"new").unwrap();
        let mut left: Vec<String> = fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        left.sort();
        assert_eq!(
            left,
            [".gone.hl7.00000000000000cc.bridgelab-tmp", ".m.hl7.00000000000000bb.bridgelab-tmp", ".m.hl7.tmp", "m.hl7"],
        );
        assert_eq!(fs::read(&p).unwrap(), b"new");
        fs::remove_dir_all(&d).ok();
    }

    #[cfg(target_os = "linux")]
    const ACL: &str = "system.posix_acl_access";

    /// A POSIX access ACL in the kernel's xattr form: version 2, then
    /// (tag, permissions, id) per entry, sorted by tag.
    #[cfg(target_os = "linux")]
    fn posix_acl(entries: &[(u16, u16, u32)]) -> Vec<u8> {
        let mut acl = 2u32.to_le_bytes().to_vec();
        for (tag, perm, id) in entries {
            acl.extend(tag.to_le_bytes());
            acl.extend(perm.to_le_bytes());
            acl.extend(id.to_le_bytes());
        }
        acl
    }

    #[cfg(target_os = "linux")]
    fn mode(p: &std::path::Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(p).unwrap().permissions().mode() & 0o7777
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn extended_attributes_and_mode_are_kept() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("xattr");
        let p = d.join("m.hl7");
        fs::write(&p, b"old").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o640)).unwrap();
        if xattr::set(&p, "user.department", b"radiology").is_err() {
            fs::remove_dir_all(&d).ok();
            return; // a file system that refuses user attributes
        }
        write_atomic(&p, b"new").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        assert_eq!(xattr::get(&p, "user.department").unwrap().as_deref(), Some(&b"radiology"[..]));
        assert_eq!(mode(&p), 0o640);
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1, "no temporary file left");
        fs::remove_dir_all(&d).ok();
    }

    /// The ACL survives, and the owning group gains nothing: the mode shows
    /// the ACL mask (rw) in its group bits, which on a file without the
    /// ACL would let the group write a file it had no access to.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_acl_is_kept_and_the_owning_group_gains_no_access() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("acl");
        let p = d.join("m.hl7");
        fs::write(&p, b"old").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        // user::rw-, user:65534:rw-, group::---, mask::rw-, other::---
        let acl = posix_acl(&[(0x01, 6, u32::MAX), (0x02, 6, 65534), (0x04, 0, u32::MAX), (0x10, 6, u32::MAX), (0x20, 0, u32::MAX)]);
        if xattr::set(&p, ACL, &acl).is_err() {
            fs::remove_dir_all(&d).ok();
            return; // a file system without POSIX ACLs
        }
        let before = xattr::get(&p, ACL).unwrap();
        assert_eq!(mode(&p), 0o660, "the group bits show the mask");
        write_atomic(&p, b"new").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        assert_eq!(xattr::get(&p, ACL).unwrap(), before, "the ACL, group::--- included, is kept");
        assert_eq!(mode(&p), 0o660);
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1, "no temporary file left");
        fs::remove_dir_all(&d).ok();
    }

    /// A file without an ACL does not get the one a default ACL on its
    /// folder gives every new file there.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_without_an_acl_gets_none_from_its_folder() {
        use std::os::unix::fs::PermissionsExt;
        let d = dir("acl-default");
        let p = d.join("m.hl7");
        fs::write(&p, b"old").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        // user::rwx, user:65534:rw-, group::r-x, mask::rwx, other::r-x
        let acl = posix_acl(&[(0x01, 7, u32::MAX), (0x02, 6, 65534), (0x04, 5, u32::MAX), (0x10, 7, u32::MAX), (0x20, 5, u32::MAX)]);
        if xattr::set(&d, "system.posix_acl_default", &acl).is_err() {
            fs::remove_dir_all(&d).ok();
            return; // a file system without POSIX ACLs
        }
        write_atomic(&p, b"new").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"new");
        assert_eq!(xattr::get(&p, ACL).unwrap(), None);
        assert_eq!(mode(&p), 0o600);
        fs::remove_dir_all(&d).ok();
    }
}
