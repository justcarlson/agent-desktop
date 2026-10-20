use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

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
    let pending = path.with_extension("tmp");
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&pending)
        .and_then(|mut file| file.write_all(bytes))
        .and_then(|()| std::fs::rename(&pending, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    written
}
