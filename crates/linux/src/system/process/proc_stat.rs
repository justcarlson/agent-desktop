use super::process_facts;
use crate::system::process::proc_state::ProcState;
use crate::system::process::process_lookup::Lookup;

const START_TIME_FIELD: usize = 22;
const FIELDS_BEFORE_STATE: usize = 3;

/// What one stat line says: the state and the start time in clock ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcStat {
    pub(crate) state: ProcState,
    pub(crate) start_time: u64,
}

/// Parses the text of `/proc/<pid>/stat`. The command name sits in
/// parentheses and may itself contain spaces and parentheses, so the fields
/// are counted from the last closing parenthesis; text without one, with the
/// parentheses reversed, or too short to hold a start time is rejected.
pub(crate) fn parse(stat: &str) -> Option<ProcStat> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    if close < open {
        return None;
    }
    let mut fields = stat[close + 1..].split_whitespace();
    let state = ProcState::from_letter(fields.next()?);
    let start_time = fields
        .nth(START_TIME_FIELD - FIELDS_BEFORE_STATE - 1)?
        .parse()
        .ok()?;
    Some(ProcStat { state, start_time })
}

/// What a read of `/proc/<pid>/stat` found: the parse, `None` for a process
/// that has gone, or `Err` for a read that failed for another reason or whose
/// text holds no start time.
pub(crate) fn read_stat(pid: u32) -> Result<Option<ProcStat>, ()> {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(text) => parse(&text).map(Some).ok_or(()),
        Err(error) => match process_facts::lookup_for_error(&error) {
            Lookup::Gone => Ok(None),
            Lookup::Found(_) | Lookup::Unreadable => Err(()),
        },
    }
}

/// Whether the process is stopped right now. A process that cannot be read
/// is not reported as stopped.
pub(crate) fn is_stopped(pid: u32) -> bool {
    matches!(read_stat(pid), Ok(Some(stat)) if stat.state == ProcState::Stopped)
}

/// Whether the read proves absence or a zombie/dead process. A failed read
/// leaves the process's state unknown.
pub(crate) fn is_gone(read: Result<Option<ProcStat>, ()>) -> bool {
    matches!(read, Ok(None))
        || matches!(read, Ok(Some(stat)) if matches!(stat.state, ProcState::Zombie | ProcState::Dead))
}

#[cfg(test)]
#[path = "proc_stat_tests.rs"]
mod tests;
