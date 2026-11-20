//! Reading a whole file into memory, only when it is a regular file and
//! only up to a size limit.
//!
//! `std::fs::read` reads whatever the path names until it ends: a device
//! such as `/dev/zero` never does, a named pipe blocks until a writer
//! comes, and a log that is still being written grows while it is read.
//! Out of memory, the app is killed with no message.

use std::io::Read;
use std::path::Path;

/// Why a file was not read.
#[derive(Debug)]
pub enum ReadRefusal {
    /// A device, named pipe, socket or folder: not a file that has an end.
    NotRegular,
    /// Larger than the limit; `size` is how large it was found to be.
    TooLarge { size: u64 },
    /// The file could not be opened or read.
    Io(std::io::Error),
}

/// The bytes of the regular file at `path`, if it holds at most `limit`
/// of them.
///
/// The size is checked three times: on the path before it is opened (so a
/// named pipe is never opened, which would block), on the open file (the
/// path may have been replaced in between), and while reading, which stops
/// one byte past the limit (the file may still be growing).
pub fn read_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>, ReadRefusal> {
    check(&std::fs::metadata(path).map_err(ReadRefusal::Io)?, limit)?;
    let mut file = open_for_reading(path).map_err(ReadRefusal::Io)?;
    let meta = file.metadata().map_err(ReadRefusal::Io)?;
    check(&meta, limit)?;

    let mut bytes = Vec::with_capacity(meta.len() as usize);
    (&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(ReadRefusal::Io)?;
    if bytes.len() as u64 > limit {
        let size = file.metadata().map(|m| m.len()).unwrap_or(0).max(bytes.len() as u64);
        return Err(ReadRefusal::TooLarge { size });
    }
    Ok(bytes)
}

fn check(meta: &std::fs::Metadata, limit: u64) -> Result<(), ReadRefusal> {
    if !meta.is_file() {
        return Err(ReadRefusal::NotRegular);
    }
    if meta.len() > limit {
        return Err(ReadRefusal::TooLarge { size: meta.len() });
    }
    Ok(())
}

/// Opening a named pipe for reading blocks until something opens it for
/// writing. Without blocking, a path swapped for a pipe after the first
/// check is opened at once and refused by the second; on a regular file
/// the flag changes nothing.
#[cfg(unix)]
fn open_for_reading(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_reading(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

#[cfg(test)]
mod tests {
    use super::{read_regular_file, ReadRefusal};

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bl-bounded-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn reads_a_file_up_to_the_limit() {
        let d = dir("ok");
        let p = d.join("m.hl7");
        std::fs::write(&p, b"MSH|^~\\&|A\r").unwrap();
        assert_eq!(read_regular_file(&p, 11).unwrap(), b"MSH|^~\\&|A\r");
        assert!(matches!(read_regular_file(&p, 10), Err(ReadRefusal::TooLarge { size: 11 })));
        std::fs::write(&p, b"").unwrap();
        assert_eq!(read_regular_file(&p, 0).unwrap(), b"");
        assert!(matches!(read_regular_file(&d.join("gone"), 10), Err(ReadRefusal::Io(_))));
        assert!(matches!(read_regular_file(&d, 10), Err(ReadRefusal::NotRegular)));
        std::fs::remove_dir_all(&d).ok();
    }

    /// A sparse file is large on disk without taking any room: refused
    /// from its size, not read.
    #[test]
    fn a_large_file_is_refused_before_it_is_read() {
        let d = dir("sparse");
        let p = d.join("big.hl7");
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(100 * 1024 * 1024).unwrap();
        match read_regular_file(&p, 64 * 1024 * 1024) {
            Err(ReadRefusal::TooLarge { size }) => assert_eq!(size, 100 * 1024 * 1024),
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&d).ok();
    }

    #[cfg(unix)]
    #[test]
    fn devices_and_pipes_are_not_read() {
        // Both used to be read until memory ran out, or forever.
        for dev in ["/dev/zero", "/dev/null", "/dev/urandom"] {
            if std::path::Path::new(dev).exists() {
                assert!(matches!(read_regular_file(dev.as_ref(), 1024), Err(ReadRefusal::NotRegular)), "{dev}");
            }
        }
        let d = dir("fifo");
        let fifo = d.join("f.hl7");
        let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        assert!(matches!(read_regular_file(&fifo, 1024), Err(ReadRefusal::NotRegular)));
        // Through a symbolic link too.
        std::os::unix::fs::symlink("/dev/zero", d.join("zero.hl7")).unwrap();
        assert!(matches!(read_regular_file(&d.join("zero.hl7"), 1024), Err(ReadRefusal::NotRegular)));
        std::fs::remove_dir_all(&d).ok();
    }
}
