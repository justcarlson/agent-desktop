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
fn a_process_on_this_sessions_bus_belongs_to_it() {
    let ours = key(Some("unix:path=/a/bus"), Some("/a"));
    assert!(ours.owns(Some(&environ(&[
        "HOME=/h",
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/a/bus"
    ]))));
    assert!(ours.owns(Some(&environ(&["DBUS_STARTER_ADDRESS=unix:path=/a/bus"]))));
}

#[test]
fn a_process_with_this_sessions_runtime_directory_belongs_to_it() {
    let ours = key(Some("unix:path=/a/bus"), Some("/a"));
    assert!(ours.owns(Some(&environ(&["XDG_RUNTIME_DIR=/a"]))));
}

#[test]
fn a_process_of_another_session_does_not_belong() {
    let ours = key(Some("unix:path=/a/bus"), Some("/a"));
    assert!(!ours.owns(Some(&environ(&[
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/b/bus",
        "XDG_RUNTIME_DIR=/b"
    ]))));
    assert!(!ours.owns(Some(&environ(&["HOME=/h"]))));
    assert!(!ours.owns(Some(&[])));
}

#[test]
fn a_process_whose_environment_cannot_be_read_does_not_belong() {
    assert!(!key(Some("unix:path=/a/bus"), None).owns(None));
}

#[test]
fn without_a_session_of_its_own_every_process_is_in_scope() {
    assert!(key(None, None).owns(None));
    assert!(key(None, None).owns(Some(&environ(&["XDG_RUNTIME_DIR=/b"]))));
}

#[test]
fn a_prefix_of_the_value_is_not_the_value() {
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

fn this_session() -> SessionKey {
    key(Some(BUS), Some(RUNTIME))
}

fn spawn_uniquely_named(tag: &str) -> Spawned {
    use std::os::unix::fs::PermissionsExt;
    let name = format!("adsc{tag}-{}", std::process::id());
    let directory = std::env::temp_dir().join(format!("{name}-dir"));
    std::fs::create_dir_all(&directory).expect("a directory");
    let program = directory.join(&name);
    std::fs::copy("/bin/sleep", &program).expect("a copy of sleep");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("modes");
    let child = std::process::Command::new(&program)
        .arg("60")
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
fn a_process_of_another_session_is_never_found_by_name() {
    let other = spawn_uniquely_named("x");
    let elsewhere = key(
        Some("unix:path=/run/user/test-session-b/bus"),
        Some("/run/user/test-session-b"),
    );
    assert_eq!(super::find_in_session(&other.name, &elsewhere), None);
}

#[test]
fn a_process_of_this_session_is_found_by_name() {
    let own = spawn_uniquely_named("y");
    let found = super::find_in_session(&own.name, &this_session()).expect("our own child");
    assert_eq!(found.0, own.child.id());
}
