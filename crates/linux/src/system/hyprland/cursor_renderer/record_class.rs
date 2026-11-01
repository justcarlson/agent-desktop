use super::exe_identity::ExeIdentity;
use super::renderer_record::RendererRecord;
use crate::system::process::proc_stat::read_stat;
use crate::system::process::proc_state::ProcState;

/// What a caller makes of a renderer record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecordClass {
    /// The pid lives with the recorded start time and runs the caller's
    /// executable.
    Valid(RendererRecord),
    /// The pid lives with the recorded start time but runs another
    /// executable: another build's renderer. It is not stale.
    IncompatibleLive(RendererRecord),
    /// The pid is gone, a zombie, or reused by a process with another start
    /// time.
    Stale(RendererRecord),
}

impl RecordClass {
    /// Classifies `record` against the caller's executable `caller`. An
    /// executable that cannot be read counts as another build, so the caller
    /// neither signals nor replaces a live renderer it cannot identify.
    pub(crate) fn of(record: RendererRecord, caller: Option<ExeIdentity>) -> Self {
        if !is_alive_as_recorded(&record) {
            return Self::Stale(record);
        }
        let running = ExeIdentity::of_pid(record.owner.pid);
        match (running, caller) {
            (Some(running), Some(caller)) if running == caller => Self::Valid(record),
            _ => Self::IncompatibleLive(record),
        }
    }
}

/// Whether the recorded pid is alive with the recorded start time.
pub(crate) fn is_alive_as_recorded(record: &RendererRecord) -> bool {
    match read_stat(record.owner.pid) {
        Ok(Some(stat)) => {
            stat.start_time == record.owner.start_time
                && !matches!(stat.state, ProcState::Zombie | ProcState::Dead)
        }
        Ok(None) => false,
        Err(()) => true,
    }
}

#[cfg(test)]
#[path = "record_class_tests.rs"]
mod tests;
