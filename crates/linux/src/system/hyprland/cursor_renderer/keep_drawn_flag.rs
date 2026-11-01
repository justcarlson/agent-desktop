//! A test hook that keeps the cursors drawn through a `hide`, so a live run
//! can show that input passes through a drawn cursor surface. It is honored
//! only for a regular file named [`KEEP_DRAWN_FLAG`] in the registry
//! directory that the renderer's own user owns; a symlink or another
//! user's file is ignored. A `hide` it changes says so in its reply.

use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// The flag file's name inside the registry directory.
pub(crate) const KEEP_DRAWN_FLAG: &str = "keep-drawn-for-test";

/// Whether `registry` holds a flag file this user may set.
pub(crate) fn keep_drawn_for_test(registry: &Path) -> bool {
    let user = rustix::process::getuid().as_raw();
    std::fs::symlink_metadata(registry.join(KEEP_DRAWN_FLAG))
        .is_ok_and(|metadata| flag_honored(&metadata, user))
}

/// Whether a flag with `metadata`, read without following a symlink, is a
/// regular file that `user` owns.
fn flag_honored(metadata: &Metadata, user: u32) -> bool {
    metadata.file_type().is_file() && metadata.uid() == user
}

#[cfg(test)]
#[path = "keep_drawn_flag_tests.rs"]
mod tests;
