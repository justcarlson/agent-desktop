//! Every renderer record in one compositor's registry dir, classified, so a
//! hide reaches the renderers of every session on the compositor.

use std::path::Path;

use super::exe_identity::ExeIdentity;
use super::record_class::RecordClass;
use super::record_read::{RecordRead, read_record};
use super::renderer_paths::RendererPaths;

/// The classified records in the registry dir `dir`. A record that cannot
/// be read is left out; its surfaces, if any, belong to no renderer that hid
/// and so fail the layer audit.
pub(crate) fn census(dir: &Path, caller: Option<ExeIdentity>) -> Vec<(RendererPaths, RecordClass)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let path = entry.path();
            let key = path
                .file_name()?
                .to_str()?
                .strip_suffix(".json")?
                .to_owned();
            if key.is_empty() || key.contains('.') {
                return None;
            }
            let paths = RendererPaths::in_dir(dir, &key).ok()?;
            match read_record(&paths.record) {
                RecordRead::Present(record) => Some((paths, RecordClass::of(record, caller))),
                RecordRead::Absent | RecordRead::Unreadable => None,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "renderer_census_tests.rs"]
mod tests;
