//! A renderer's read of the hide holders on its compositor. It rereads the
//! directory before every draw, so a holder written by any caller takes
//! effect on the next frame. A file that does not parse is no holder.

use std::path::Path;

use super::holder_file::{HolderFile, now_ms};

/// The live holders in `holders`.
pub(crate) fn live_holders(holders: &Path) -> Vec<HolderFile> {
    let Ok(entries) = std::fs::read_dir(holders) else {
        return Vec::new();
    };
    let now = now_ms();
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| HolderFile::parse(&bytes))
        .filter(|holder| holder.is_live(now))
        .collect()
}

/// Whether any holder in `holders` is live.
pub(crate) fn any_live(holders: &Path) -> bool {
    !live_holders(holders).is_empty()
}
