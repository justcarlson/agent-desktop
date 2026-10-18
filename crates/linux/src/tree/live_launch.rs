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
        for path in self.scratch.drain(..) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

impl Drop for Launched {
    fn drop(&mut self) {
        self.stop();
    }
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
