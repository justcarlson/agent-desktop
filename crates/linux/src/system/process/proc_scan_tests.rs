use super::SessionKey;

fn key(bus: Option<&str>, runtime: Option<&str>) -> SessionKey {
    SessionKey {
        bus: bus.map(str::to_owned),
        runtime: runtime.map(str::to_owned),
    }
}

fn environ(entries: &[&str]) -> Vec<u8> {
    entries.join("\0").into_bytes()
}

#[test]
fn session_ownership_requires_an_exact_bus_or_runtime_match() {
    let ours = key(Some("unix:path=/a/bus"), Some("/a"));
    assert!(ours.owns(Some(&environ(&[
        "HOME=/h",
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/a/bus"
    ]))));
    assert!(ours.owns(Some(&environ(&["DBUS_STARTER_ADDRESS=unix:path=/a/bus"]))));
    assert!(ours.owns(Some(&environ(&["XDG_RUNTIME_DIR=/a"]))));
    assert!(!ours.owns(Some(&environ(&[
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/b/bus",
        "XDG_RUNTIME_DIR=/b"
    ]))));
    assert!(!ours.owns(Some(&environ(&["HOME=/h"]))));
    assert!(!ours.owns(Some(&[])));
    assert!(!key(Some("unix:path=/a/bus"), None).owns(None));
    assert!(key(None, None).owns(None));
    assert!(key(None, None).owns(Some(&environ(&["XDG_RUNTIME_DIR=/b"]))));
    let ours = key(None, Some("/run/user/1000"));
    assert!(!ours.owns(Some(&environ(&["XDG_RUNTIME_DIR=/run/user/10000"]))));
}

struct Spawned {
    child: std::process::Child,
    directory: std::path::PathBuf,
    name: String,
}

impl Drop for Spawned {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

const RUNTIME: &str = "/run/user/test-session-a";
const BUS: &str = "unix:path=/run/user/test-session-a/bus";
const CHILD: &str = "AGENT_DESKTOP_PROC_SCAN_CHILD";

fn this_session() -> SessionKey {
    key(Some(BUS), Some(RUNTIME))
}

fn spawn_uniquely_named(tag: &str) -> Spawned {
    use std::os::unix::fs::PermissionsExt;
    let name = format!("adsc{tag}-{}", std::process::id());
    let directory = std::env::temp_dir().join(format!("{name}-dir"));
    std::fs::create_dir_all(&directory).expect("a directory");
    let program = directory.join(&name);
    std::fs::copy(std::env::current_exe().expect("test executable"), &program)
        .expect("a copy of the test executable");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("modes");
    let child = std::process::Command::new(&program)
        .args([
            "--exact",
            "system::process::proc_scan::tests::proc_scan_finds_our_process_and_rejects_it_from_another_session",
        ])
        .env(CHILD, "1")
        .env("XDG_RUNTIME_DIR", RUNTIME)
        .env("DBUS_SESSION_BUS_ADDRESS", BUS)
        .env_remove("DBUS_STARTER_ADDRESS")
        .spawn()
        .expect("a child");
    let started = std::time::Instant::now();
    let short: String = name.chars().take(15).collect();
    while started.elapsed() < std::time::Duration::from_secs(5)
        && (std::fs::read_to_string(format!("/proc/{}/comm", child.id()))
            .map_or(true, |comm| comm.trim_end() != short)
            || std::fs::read(format!("/proc/{}/environ", child.id()))
                .map_or(true, |bytes| bytes.is_empty()))
    {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Spawned {
        child,
        directory,
        name,
    }
}

#[test]
fn proc_scan_finds_our_process_and_rejects_it_from_another_session() {
    if std::env::var_os(CHILD).is_some() {
        std::thread::sleep(std::time::Duration::from_secs(60));
        return;
    }
    let own = spawn_uniquely_named("x");
    let elsewhere = key(
        Some("unix:path=/run/user/test-session-b/bus"),
        Some("/run/user/test-session-b"),
    );
    assert_eq!(
        super::find_in_session(&own.name, &this_session())
            .unwrap()
            .0,
        own.child.id()
    );
    assert_eq!(super::find_in_session(&own.name, &elsewhere), None);
}
