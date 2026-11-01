use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// The file an executable runs from, named by device and inode, so a rebuilt
/// binary at the same path is a different executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExeIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
}

impl ExeIdentity {
    /// The identity of the file `path` resolves to.
    pub(crate) fn of_path(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    /// The identity of the file process `pid` runs, which stays readable
    /// after the file is replaced or removed.
    pub(crate) fn of_pid(pid: u32) -> Option<Self> {
        Self::of_path(Path::new(&format!("/proc/{pid}/exe")))
    }

    /// The identity of this process's executable.
    pub(crate) fn current() -> Option<Self> {
        Self::of_path(Path::new("/proc/self/exe"))
    }
}
