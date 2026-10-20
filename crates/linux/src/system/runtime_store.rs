use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PENDING: AtomicU64 = AtomicU64::new(0);

/// A directory of this name in the runtime directory, or `None` when the
/// session has no runtime directory.
pub(super) fn runtime_subdir(name: &str) -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())?;
    Some(PathBuf::from(runtime).join(name))
}

/// The file stem of an entry kept for one client and one process instance:
/// the client in hexadecimal and the instance token with every character that
/// is not alphanumeric replaced.
pub(super) fn instance_stem(client: u32, instance: &str) -> String {
    let token: String = instance
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{client:08x}-{token}")
}

fn create_pending(path: &Path) -> std::io::Result<(PathBuf, std::fs::File)> {
    loop {
        let sequence = NEXT_PENDING.fetch_add(1, Ordering::Relaxed);
        let pending = path.with_extension(format!("{}.{sequence}.tmp", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&pending)
        {
            Ok(file) => return Ok((pending, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Writes `bytes` to `path` through a pending file beside it that is renamed
/// into place, in a private directory (mode 0700, file 0600). A failed write
/// leaves no pending file behind.
pub(super) fn write_private_atomically(
    dir: &Path,
    path: &Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let (pending, mut file) = create_pending(path)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| std::fs::rename(&pending, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    written
}

#[cfg(test)]
#[path = "runtime_store_tests.rs"]
mod tests;
