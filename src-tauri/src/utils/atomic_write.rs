//! Replace a file without ever leaving it half-written.
//!
//! `fs::write` truncates the target first and then writes: if the disk
//! fills or the process dies in between, the original is gone and a
//! partial file is left behind (audit finding: a failed Save destroyed the
//! message it was saving). Here the new content goes to a temporary file in
//! the same directory, is flushed to disk, and only then renamed over the
//! target in one step. On any error the target is untouched.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Create a fresh temporary file next to `target`: an unpredictable name,
/// opened with create_new so an existing file or a planted symlink is
/// never followed or truncated. Retries on a name collision.
fn create_temp(dir: &Path, name: &str) -> io::Result<(PathBuf, File)> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    for _ in 0..32 {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let salt = (nanos as u64) ^ (std::process::id() as u64).rotate_left(32) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let tmp = dir.join(format!(".{}.{:016x}.bridgelab-tmp", name, salt));
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => return Ok((tmp, f)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "could not create a unique temporary file"))
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
/// - a hard-linked file (Unix): a new inode would leave the other names
///   holding the old text;
/// - a file whose owner or group we cannot give the new file (someone
///   else's file we may write to through the group): replacing it would
///   hand it to us and could lock its owner out;
/// - a writable file in a folder we cannot create files in, which the
///   pre-1.9 plain write could save and the temporary file cannot.
///
/// ACLs and extended attributes (macOS Finder tags) are not carried over
/// by the replace; they are kept only on the in-place path.
pub fn write_atomic(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    let path = path.as_ref();
    let target = resolve_target(path);
    let existing = fs::metadata(&target).ok().filter(|m| m.is_file());
    if let Some(meta) = &existing {
        refuse_read_only(&target, meta)?;
    }
    if existing.as_ref().is_some_and(is_hard_linked) {
        return write_in_place(&target, bytes);
    }
    let dir = target.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let (tmp, mut f) = match create_temp(dir, &name) {
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
        if let Some(meta) = &existing {
            if !copy_owner(meta, &tmp) {
                return Ok(false);
            }
            let _ = fs::set_permissions(&tmp, meta.permissions());
        }
        // Replaces an existing file on every OS (MoveFileEx with
        // MOVEFILE_REPLACE_EXISTING on Windows).
        fs::rename(&tmp, &target).map(|_| true)
    })();
    match result {
        Ok(true) => Ok(()),
        Ok(false) => {
            let _ = fs::remove_file(&tmp);
            write_in_place(&target, bytes)
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
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
fn is_hard_linked(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() > 1
}

#[cfg(not(unix))]
fn is_hard_linked(_meta: &fs::Metadata) -> bool {
    // Windows reports the link count only through an unstable API; hard
    // links to message files are rare there.
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

    #[cfg(unix)]
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
}
