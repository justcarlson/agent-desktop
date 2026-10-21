use std::path::Path;
use std::sync::OnceLock;

use super::proc_stat;

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

pub(crate) const ESRCH: i32 = 3;

/// Whether a failed `/proc` read says the process has exited, as opposed to
/// the read having failed for another reason.
pub(crate) fn lookup_for_error(error: &std::io::Error) -> Lookup {
    if error.kind() == std::io::ErrorKind::NotFound || error.raw_os_error() == Some(ESRCH) {
        Lookup::Gone
    } else {
        Lookup::Unreadable
    }
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

/// The token for a start time on this boot.
pub(crate) fn instance_token(start_time: u64) -> String {
    process_instance(boot_id(), start_time)
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
    inspect(pid, true).ok()
}

/// Looks one process up. A made-up start time would let a recycled id pass,
/// so a stat line without a readable start time is `Unreadable`, and only a
/// read that failed because the process has gone is `Gone`.
pub(crate) fn lookup(pid: u32) -> Lookup {
    match inspect(pid, true) {
        Ok(facts) => Lookup::Found(facts),
        Err(lookup) => lookup,
    }
}

/// The facts a name is matched on: the same as [`read`] without the check for
/// a Chromium payload next to the executable, which a name never needs.
pub(crate) fn read_names(pid: u32) -> Option<ProcessFacts> {
    inspect(pid, false).ok()
}

fn inspect(pid: u32, check_payload: bool) -> Result<ProcessFacts, Lookup> {
    let stat = match proc_stat::read_stat(pid) {
        Ok(Some(stat)) => stat,
        Ok(None) => return Err(Lookup::Gone),
        Err(()) => return Err(Lookup::Unreadable),
    };
    let comm = match std::fs::read_to_string(format!("/proc/{pid}/comm")) {
        Ok(comm) => comm.trim_end().to_owned(),
        Err(error) => return Err(lookup_for_error(&error)),
    };
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok();
    let exe_basename = exe
        .as_deref()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    let payload = check_payload && exe.as_deref().is_some_and(has_chromium_payload);
    let facts = ProcessFacts {
        web_family: classify_web(&comm, exe_basename.as_deref(), payload),
        comm,
        exe_basename,
        instance: process_instance(boot_id(), stat.start_time),
    };
    Ok(facts)
}

#[cfg(test)]
#[path = "process_facts_tests.rs"]
mod tests;
