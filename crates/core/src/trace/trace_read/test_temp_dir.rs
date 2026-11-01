use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "agent-desktop-{prefix}-{}-{}",
            std::process::id(),
            crate::refs::new_snapshot_id()
        ));
        let directory = Self { path };
        crate::storage::private_file_parent::ensure_private(&directory.path).unwrap();
        directory
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
