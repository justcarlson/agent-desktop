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

/// The facts of a running process, or `None` when it has gone or its start
/// time cannot be read: a made-up token would let a recycled id pass.
pub(crate) fn read(pid: u32) -> Option<ProcessFacts> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let start_time = parse_start_time(&stat)?;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()?
        .trim_end()
        .to_owned();
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).ok();
    let exe_basename = exe
        .as_deref()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    let payload = exe.as_deref().is_some_and(has_chromium_payload);
    Some(ProcessFacts {
        web_family: classify_web(&comm, exe_basename.as_deref(), payload),
        comm,
        exe_basename,
        instance: process_instance(boot_id(), start_time),
    })
}

#[cfg(test)]
#[path = "process_facts_tests.rs"]
mod tests;
