use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

const PRIVATE_DIR_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

fn current_user() -> u32 {
    rustix::process::getuid().as_raw()
}

/// Creates `dir` with mode 0700, or accepts an existing one only when this
/// user owns it and no one else can enter it.
pub(crate) fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIR_MODE)
        .create(dir)?;
    let metadata = fs::metadata(dir)?;
    let private = metadata.is_dir()
        && metadata.uid() == current_user()
        && metadata.permissions().mode() & 0o077 == 0;
    if private {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the clipboard state directory is not private to this user",
        ))
    }
}

/// Writes `name` in `dir` as a new 0600 file. The bytes go to a temporary
/// file first and reach the final name by a hard link, so a reader never
/// sees a partial record and an existing file is never replaced.
pub(crate) fn write_new(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let temporary = dir.join(format!(".{name}.tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(&temporary)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let linked = written.and_then(|()| fs::hard_link(&temporary, dir.join(name)));
    let _ = fs::remove_file(&temporary);
    linked
}

pub(crate) fn remove(dir: &Path, name: &str) -> io::Result<()> {
    match fs::remove_file(dir.join(name)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
#[path = "owner_files_tests.rs"]
mod tests;
