use std::path::Path;

use crate::AppError;

/// Creates the session directory and its trace directory. The state root
/// holds every session's files, so it must be private before a session is
/// written into it: a root that group or other users can reach is refused
/// here exactly as it is for a snapshot.
pub(super) fn create(dir: &Path) -> Result<(), AppError> {
    crate::private_file_parent::ensure_private(
        &crate::state_root::resolve_configured_state_root()?
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir.join("trace"))?;
    }
    #[cfg(not(unix))]
    {
        crate::private_file_parent::ensure_private(&dir.join("trace"))?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "tree_tests.rs"]
mod tests;
