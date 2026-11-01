//! Reading a renderer record, and removing one only for its own generation.

use std::path::Path;

use super::renderer_paths::RendererPaths;
use super::renderer_record::RendererRecord;

const MAX_RECORD_BYTES: u64 = 4 * 1024;

/// What a record file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordRead {
    Absent,
    Present(RendererRecord),
    Unreadable,
}

/// Reads the record at `path`.
pub(crate) fn read_record(path: &Path) -> RecordRead {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return RecordRead::Absent,
        Err(_) => return RecordRead::Unreadable,
    };
    let mut bytes = Vec::new();
    if file
        .take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_RECORD_BYTES
    {
        return RecordRead::Unreadable;
    }
    RendererRecord::parse(&bytes).map_or(RecordRead::Unreadable, RecordRead::Present)
}

/// Removes the record and the socket only when the record on disk still
/// names `generation`, so a newer renderer's files stay. Returns whether it
/// removed them.
pub(crate) fn remove_if_generation(paths: &RendererPaths, generation: &str) -> bool {
    match read_record(&paths.record) {
        RecordRead::Present(record) if record.owner.generation == generation => {
            let _ = std::fs::remove_file(&paths.socket);
            let _ = std::fs::remove_file(&paths.record);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "record_read_tests.rs"]
mod tests;
