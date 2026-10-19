use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const DIR_NAME: &str = "agent-desktop-actions";
const ANY: &str = "any";
const NO_TOKEN: &str = "-";
const KEEP_MS: u64 = 60_000;

fn runtime_dir() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    let base = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty())?;
    Some(Path::new(&base).join(DIR_NAME))
}

/// Milliseconds since the machine booted, suspend included, so two processes
/// agree on it and a restart cannot make an old entry look new.
fn boot_ms() -> Option<u64> {
    let text = fs::read_to_string("/proc/uptime").ok()?;
    let seconds: f64 = text.split_whitespace().next()?.parse().ok()?;
    Some((seconds * 1000.0) as u64)
}

fn token_of(pid: u32) -> Option<String> {
    super::process_facts::read(pid).map(|facts| facts.instance)
}

fn encode(at_ms: u64, token: &str) -> String {
    format!("{at_ms} {token}\n")
}

fn parse(text: &str) -> Option<(u64, &str)> {
    let mut parts = text.split_whitespace();
    let at = parts.next()?.parse().ok()?;
    Some((at, parts.next()?))
}

fn is_fresh(at_ms: u64, now_ms: u64, window_ms: u64) -> bool {
    now_ms >= at_ms && now_ms - at_ms < window_ms
}

fn name_of(target: Option<u32>) -> String {
    target.map_or_else(|| ANY.to_owned(), |pid| pid.to_string())
}

fn write_atomic(dir: &Path, name: &str, body: &str) -> std::io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    let scratch = dir.join(format!(".{name}.{}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&scratch)?;
    file.write_all(body.as_bytes())?;
    drop(file);
    fs::rename(&scratch, dir.join(name))
}

fn prune(dir: &Path, now_ms: u64, token_now: &dyn Fn(u32) -> Option<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let text = fs::read_to_string(entry.path()).unwrap_or_default();
        let stale = match parse(&text) {
            None => true,
            Some((at, token)) => {
                !is_fresh(at, now_ms, KEEP_MS)
                    || name
                        .parse::<u32>()
                        .is_ok_and(|pid| token_now(pid).as_deref() != Some(token))
            }
        };
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

pub(super) fn record_in(
    dir: &Path,
    target: Option<u32>,
    now_ms: u64,
    token_now: &dyn Fn(u32) -> Option<String>,
) {
    let token = target
        .and_then(token_now)
        .unwrap_or_else(|| NO_TOKEN.to_owned());
    prune(dir, now_ms, token_now);
    let _ = write_atomic(dir, &name_of(target), &encode(now_ms, &token));
}

pub(super) fn recent_in(
    dir: &Path,
    target: Option<u32>,
    now_ms: u64,
    window_ms: u64,
    token_now: &dyn Fn(u32) -> Option<String>,
) -> bool {
    let Ok(text) = fs::read_to_string(dir.join(name_of(target))) else {
        return false;
    };
    let Some((at, token)) = parse(&text) else {
        return false;
    };
    let same_process = target.is_none_or(|pid| token_now(pid).as_deref() == Some(token));
    same_process && is_fresh(at, now_ms, window_ms)
}

/// Writes that an action was just delivered, for the whole session when
/// `target` is `None` and for one process otherwise. A directory that cannot
/// be written leaves no record, which reads as no recent action.
pub(super) fn record(target: Option<u32>) {
    if let (Some(dir), Some(now)) = (runtime_dir(), boot_ms()) {
        record_in(&dir, target, now, &token_of);
    }
}

pub(super) fn recent(target: Option<u32>, window_ms: u64) -> bool {
    match (runtime_dir(), boot_ms()) {
        (Some(dir), Some(now)) => recent_in(&dir, target, now, window_ms, &token_of),
        _ => false,
    }
}

#[cfg(test)]
#[path = "action_clock_store_tests.rs"]
mod tests;
