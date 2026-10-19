use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub(crate) struct Launched {
    children: Vec<Child>,
    pub(crate) scratch: Vec<PathBuf>,
}

impl Launched {
    /// A launcher for apps in a private session. Apps that D-Bus activates
    /// take their display from the bus's activation environment, not from this
    /// process, so a launcher outside the session helper could map windows on
    /// the owner's screen; it refuses to start there.
    pub(crate) fn new() -> Self {
        assert!(
            std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some(),
            "live launches must run inside scripts/private-session.sh"
        );
        Self {
            children: Vec::new(),
            scratch: Vec::new(),
        }
    }

    pub(crate) fn spawn(&mut self, program: &str, args: &[&str]) -> u32 {
        self.spawn_with_env(program, args, &[])
    }

    pub(crate) fn spawn_with_env(
        &mut self,
        program: &str,
        args: &[&str],
        env: &[(&str, String)],
    ) -> u32 {
        use std::os::unix::process::CommandExt;
        let child = Command::new(program)
            .args(args)
            .envs(env.iter().map(|(key, value)| (*key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap_or_else(|error| panic!("could not launch {program}: {error}"));
        let pid = child.id();
        self.children.push(child);
        pid
    }

    pub(crate) fn stop(&mut self) {
        for child in &mut self.children {
            let group = format!("-{}", child.id());
            let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
            let _ = child.kill();
            let _ = child.wait();
        }
        self.children.clear();
        stop_activated_servers();
        for path in self.scratch.drain(..) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

const ACTIVATED_SERVERS: [&str; 1] = ["gnome-terminal-"];

/// Ends the servers that D-Bus started for a launched client and that outlive
/// it: `gnome-terminal` only asks its server to open a window, so killing the
/// client leaves the server and its window for every later test of the
/// session. Only processes of this session's own runtime directory are
/// signalled, each by pid.
fn stop_activated_servers() {
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let wanted = format!("XDG_RUNTIME_DIR={}", runtime.to_string_lossy());
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let comm = std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
        if !is_activated_server(comm.trim()) {
            continue;
        }
        let environ = std::fs::read(entry.path().join("environ")).unwrap_or_default();
        let ours = environ
            .split(|byte| *byte == 0)
            .any(|variable| variable == wanted.as_bytes());
        if ours {
            let _ = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status();
        }
    }
}

fn is_activated_server(comm: &str) -> bool {
    ACTIVATED_SERVERS
        .iter()
        .any(|prefix| comm.starts_with(prefix))
}

impl Drop for Launched {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The command line the live tests give VS Code. `--password-store=basic` is
/// required in a private session: its keyring starts empty, so the first
/// secrets request makes the keyring daemon open a password prompt nobody
/// answers, and the browser's UI thread, which also serves the accessibility
/// bus, blocks on that request and its frame leaves the registry (`L19-4`).
pub(crate) fn vscode_arguments(user_data_dir: &str) -> [&str; 6] {
    [
        "--no-sandbox",
        "--password-store=basic",
        "--user-data-dir",
        user_data_dir,
        "--disable-extensions",
        "--new-window",
    ]
}

pub(crate) fn nonce(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("ad-live-{label}-{}-{nanos}", std::process::id())
}

pub(crate) fn program_of(pid: u32) -> Option<String> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let argv0 = String::from_utf8_lossy(bytes.split(|byte| *byte == 0).next()?).into_owned();
    argv0.rsplit('/').next().map(str::to_owned)
}

/// Box facts that decide whether a tree that never filled is the application's fault
/// or the machine's: load average and memory and swap headroom.
pub(crate) fn environment_note() -> String {
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| {
            text.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
                .into()
        })
        .unwrap_or_else(|| "unreadable".to_owned());
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let kib = |key: &str| {
        meminfo
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.trim_start_matches(':').split_whitespace().next())
            .map_or_else(
                || "?".to_owned(),
                |value| format!("{} MiB", value.parse::<u64>().map_or(0, |kib| kib / 1024)),
            )
    };
    format!(
        "box load {load}, MemAvailable {}, SwapFree {}",
        kib("MemAvailable"),
        kib("SwapFree")
    )
}

#[cfg(test)]
mod activated_tests {
    use super::is_activated_server;

    #[test]
    fn only_the_terminal_server_is_an_activated_server() {
        assert!(is_activated_server("gnome-terminal-"));
        assert!(!is_activated_server("gnome-terminal"));
        assert!(!is_activated_server("dbus-daemon"));
        assert!(!is_activated_server("Xvfb"));
        assert!(!is_activated_server("openbox"));
    }
}
