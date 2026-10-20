use super::process_facts::{self, Lookup};

const START_TIME_FIELD: usize = 22;
const FIELDS_BEFORE_STATE: usize = 3;

/// The state letter of `/proc/<pid>/stat`, as a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcState {
    Running,
    Sleeping,
    DiskSleep,
    Stopped,
    Zombie,
    Dead,
    Idle,
    Other,
}

impl ProcState {
    fn from_letter(letter: &str) -> Self {
        match letter {
            "R" => Self::Running,
            "S" => Self::Sleeping,
            "D" => Self::DiskSleep,
            "T" | "t" => Self::Stopped,
            "Z" => Self::Zombie,
            "X" | "x" => Self::Dead,
            "I" => Self::Idle,
            _ => Self::Other,
        }
    }
}

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

#[cfg(test)]
#[path = "proc_stat_tests.rs"]
mod tests;
