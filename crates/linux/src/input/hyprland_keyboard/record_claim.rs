//! A claim on the journal record a dead delivery left. The key guard of that
//! delivery and the next delivery's recovery both release the keys it held,
//! and Hyprland takes a key-up from any virtual keyboard, so a second, late
//! release could lift a key the next command holds. Each one claims the record
//! under an exclusive `flock` on the record file and keeps the claim until it
//! has sent the key-ups and removed the file. Whoever claims second finds the
//! file gone and sends nothing. The claim never touches the interaction lease,
//! which a host may hold for the whole run.

use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_desktop_core::Deadline;
use rustix::fs::{FlockOperation, flock};
use rustix::io::Errno;
use serde_json::Value;

const CLAIM_POLL: Duration = Duration::from_millis(5);

/// An open record file this process holds the exclusive lock on.
#[derive(Debug)]
pub(crate) struct RecordClaim {
    file: File,
    path: PathBuf,
}

/// Claims the record at `path`, waiting until `deadline` for a claim in
/// progress. `None` means the record is gone: never written, or released and
/// removed by whoever claimed it first. A claim still held at the deadline is
/// a `TimedOut` error, and the caller must send nothing.
pub(crate) fn claim_record(path: &Path, deadline: Deadline) -> io::Result<Option<RecordClaim>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    lock_until(&file, deadline)?;
    let held = file.metadata()?;
    match std::fs::metadata(path) {
        Ok(named) if named.dev() == held.dev() && named.ino() == held.ino() => {
            Ok(Some(RecordClaim {
                file,
                path: path.to_owned(),
            }))
        }
        Ok(_) => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn lock_until(file: &File, deadline: Deadline) -> io::Result<()> {
    loop {
        match flock(file, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => return Ok(()),
            Err(errno) if errno == Errno::WOULDBLOCK || errno == Errno::INTR => {}
            Err(errno) => return Err(errno.into()),
        }
        if deadline.is_expired() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Another release still holds the held-key record",
            ));
        }
        std::thread::sleep(CLAIM_POLL.min(deadline.remaining()));
    }
}

impl RecordClaim {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The record as the dead delivery last wrote it.
    pub(crate) fn value(&self) -> io::Result<Value> {
        serde_json::from_reader(&self.file).map_err(io::Error::other)
    }

    /// Removes the record once its keys are released, then gives up the
    /// claim. A record already gone counts as removed.
    pub(crate) fn remove(self) -> io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "record_claim_tests.rs"]
mod tests;
