use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[derive(PartialEq, Eq)]
pub(crate) struct DesktopStamp {
    path: PathBuf,
    version: Option<(u64, u64, i64, i64)>,
}

impl DesktopStamp {
    pub(super) fn read(path: PathBuf) -> Self {
        let version = std::fs::metadata(&path).ok().map(|metadata| {
            (
                metadata.ino(),
                metadata.len(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            )
        });
        Self { path, version }
    }
}
