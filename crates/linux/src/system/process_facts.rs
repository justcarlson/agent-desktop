use std::path::Path;
use std::sync::OnceLock;

const START_TIME_FIELD: usize = 22;
const FIELDS_BEFORE_STATE: usize = 3;
const BOOT_ID_PREFIX_LEN: usize = 8;
const CHROMIUM_NAMES: [&str; 5] = ["code", "chrome", "chromium", "electron", "google-chrome"];
const GECKO_NAMES: [&str; 3] = ["firefox", "firefox-bin", "firefox-esr"];
const CHROMIUM_PAYLOAD: [&str; 2] = ["resources/app.asar", "libffmpeg.so"];

/// A browser engine family whose accessibility tree fills in after launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebFamily {
    Chromium,
    Gecko,
}

/// What `/proc` knows about one process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessFacts {
    pub(crate) comm: String,
    pub(crate) exe_basename: Option<String>,
    pub(crate) instance: String,
    pub(crate) web_family: Option<WebFamily>,
}

/// What a lookup of one process id found. A process that is gone and one
/// that could not be read are different answers: only the first may end a
/// ref's life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Lookup {
    Found(ProcessFacts),
    Gone,
    Unreadable,
}

impl Lookup {
    pub(crate) fn found(self) -> Option<ProcessFacts> {
        match self {
            Self::Found(facts) => Some(facts),
            Self::Gone | Self::Unreadable => None,
        }
    }
}

/// Whether a failed `/proc` read says the process has exited, as opposed to
/// the read having failed for another reason.
pub(crate) fn lookup_for_error(error: &std::io::Error) -> Lookup {
    const ESRCH: i32 = 3;
    if error.kind() == std::io::ErrorKind::NotFound || error.raw_os_error() == Some(ESRCH) {
        Lookup::Gone
    } else {
        Lookup::Unreadable
    }
}

/// The start time, in clock ticks since boot, from the text of
/// `/proc/<pid>/stat`. The command name sits in parentheses and may itself
/// contain spaces and parentheses, so the fields are counted from the last
/// closing parenthesis; text without one is rejected.
pub(crate) fn parse_start_time(stat: &str) -> Option<u64> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    if close < open {
        return None;
    }
    stat[close + 1..]
        .split_whitespace()
        .nth(START_TIME_FIELD - FIELDS_BEFORE_STATE)?
        .parse()
        .ok()
}

/// Whether the state letter of `/proc/<pid>/stat` says the process is
/// stopped by a signal (`T`) or by a tracer (`t`). The state follows the last
/// closing parenthesis, like the start time.
pub(crate) fn parse_stopped(stat: &str) -> bool {
    stat.rfind(')')
        .and_then(|close| stat[close + 1..].split_whitespace().next())
        .is_some_and(|state| state == "T" || state == "t")
}

/// Whether the process is stopped right now. A process that cannot be read is
/// not reported as stopped.
pub(crate) fn is_stopped(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| parse_stopped(&stat))
}

/// The token that tells two runs of one process id apart: a prefix of the
/// boot id, so it differs across reboots, and the start time.
pub(crate) fn process_instance(boot_id: &str, start_time: u64) -> String {
    let prefix: String = boot_id.trim().chars().take(BOOT_ID_PREFIX_LEN).collect();
    format!("{prefix}:{start_time}")
}

/// The engine family of a process, from its command name, executable name,
/// and whether a Chromium payload sits beside the executable. The payload
/// signal classifies an application that ships under its own binary name.
pub(crate) fn classify_web(
    comm: &str,
    exe_basename: Option<&str>,
    chromium_payload: bool,
) -> Option<WebFamily> {
    let names = [Some(comm), exe_basename];
    let named = |list: &[&str]| {
        names
            .iter()
            .flatten()
            .any(|name| list.iter().any(|known| name.eq_ignore_ascii_case(known)))
    };
    if named(&CHROMIUM_NAMES) || chromium_payload {
        Some(WebFamily::Chromium)
    } else if named(&GECKO_NAMES) {
        Some(WebFamily::Gecko)
    } else {
        None
    }
}

fn boot_id() -> &'static str {
    static BOOT_ID: OnceLock<String> = OnceLock::new();
    BOOT_ID.get_or_init(|| {
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap_or_default()
    })
}

fn has_chromium_payload(exe: &Path) -> bool {
    exe.parent().is_some_and(|dir| {
        CHROMIUM_PAYLOAD
            .iter()
            .any(|relative| dir.join(relative).exists())
    })
}

/// The facts of a running process, or `None` when it has gone or could not be
/// read.
pub(crate) fn read(pid: u32) -> Option<ProcessFacts> {
    lookup(pid).found()
}

/// Looks one process up. A made-up start time would let a recycled id pass,
/// so a stat line without a readable start time is `Unreadable`, and only a
/// read that failed because the process has gone is `Gone`.
pub(crate) fn lookup(pid: u32) -> Lookup {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) => return lookup_for_error(&error),
    };
    let Some(start_time) = parse_start_time(&stat) else {
        return Lookup::Unreadable;
    };
    let comm = match std::fs::read_to_string(format!("/proc/{pid}/comm")) {
        Ok(comm) => comm.trim_end().to_owned(),
        Err(error) => return lookup_for_error(&error),
    };
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok();
    let exe_basename = exe
        .as_deref()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    let payload = exe.as_deref().is_some_and(has_chromium_payload);
    Lookup::Found(ProcessFacts {
        web_family: classify_web(&comm, exe_basename.as_deref(), payload),
        comm,
        exe_basename,
        instance: process_instance(boot_id(), start_time),
    })
}

#[cfg(test)]
#[path = "process_facts_tests.rs"]
mod tests;
