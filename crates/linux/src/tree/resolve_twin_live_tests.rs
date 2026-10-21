use agent_desktop_core::{Deadline, ObservationOps, RefEntry};

use super::live_launch::Launched;
use super::live_tree::pid_frame;
use super::resolve_live_tests::snapshot_entries;
use super::walker_live_tests::settle;
use super::{AtspiElement, atspi_element};
use crate::LinuxAdapter;
use crate::system::SharedBus;

fn independently_answers(frame: &AtspiElement) -> bool {
    let address = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.a11y.Bus",
            "--object-path",
            "/org/a11y/bus",
            "--method",
            "org.a11y.Bus.GetAddress",
            "--timeout",
            "2",
        ])
        .output()
        .expect("bus address oracle");
    let text = String::from_utf8_lossy(&address.stdout);
    let address = text.split('\'').nth(1).expect("quoted bus address");
    std::process::Command::new("gdbus")
        .args([
            "call",
            "--address",
            address,
            "--dest",
            frame.bus_name(),
            "--object-path",
            frame.path(),
            "--method",
            "org.a11y.atspi.Accessible.GetRole",
            "--timeout",
            "2",
        ])
        .output()
        .expect("independent frame read")
        .status
        .success()
}

fn named(entries: Vec<(String, RefEntry)>, name: &str) -> Vec<RefEntry> {
    entries
        .into_iter()
        .map(|(_, entry)| entry)
        .filter(|entry| entry.identity.name.as_deref() == Some(name))
        .collect()
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn resolve_live_files_row_refs_survive_a_sibling_row_leaving() {
    use super::live_launch::nonce;
    use super::live_rig::wait_until;
    use std::time::Duration;

    let adapter = LinuxAdapter::new();
    let bus = SharedBus::default();
    let connection = bus
        .get(Deadline::standard().expect("a standard deadline builds"))
        .expect("the accessibility bus must be reachable");
    let mut launched = Launched::new();
    let folder = std::env::temp_dir().join(nonce("resolve-rows"));
    std::fs::create_dir_all(&folder).expect("the folder is made");
    launched.scratch.push(folder.clone());
    for name in ["alpha.txt", "beta.txt"] {
        std::fs::write(folder.join(name), b"x").expect("a file is written");
    }
    let pid = launched.spawn("nautilus", &["--new-window", &folder.to_string_lossy()]);
    let settled = settle(&bus, 30, || pid_frame(&connection, pid)).expect("files settles");
    let snapshot = |name: &str| {
        named(
            snapshot_entries(&adapter, &settled.frame, false).expect("a snapshot"),
            name,
        )
    };
    let mut before = Vec::new();
    assert!(
        wait_until(Duration::from_secs(20), || {
            before = snapshot("beta.txt");
            !before.is_empty() && !snapshot("alpha.txt").is_empty()
        }),
        "Files lists both files"
    );
    println!("resolve_live files rows: beta refs={}", before.len());
    let removed = snapshot("alpha.txt");
    std::fs::remove_file(folder.join("alpha.txt")).expect("the first file is removed");
    assert!(
        wait_until(Duration::from_secs(20), || snapshot("alpha.txt").is_empty()),
        "Files stops listing the removed file"
    );
    assert!(!folder.join("alpha.txt").exists());
    assert!(
        independently_answers(&settled.frame),
        "Files still answers D-Bus"
    );
    for mut entry in removed {
        entry.scope.path.clear();
        let error = adapter
            .resolve_element_strict(&entry, Deadline::after(5_000).unwrap())
            .err()
            .expect("removed row");
        assert_eq!(error.code, agent_desktop_core::ErrorCode::StaleRef);
    }
    let mut paths = Vec::new();
    for entry in &before {
        let handle = adapter
            .resolve_element_strict(entry, Deadline::after(5_000).expect("the deadline builds"))
            .unwrap_or_else(|error| {
                panic!(
                    "a row ref {:?} {:?} did not survive the removal: {:?} {}",
                    entry.identity.role, entry.scope.path, error.code, error.message
                )
            });
        paths.push(atspi_element(&handle).expect("ours").path().to_owned());
    }
    paths.sort();
    paths.dedup();
    assert_eq!(
        paths.len(),
        before.len(),
        "each ref resolves to its own node"
    );
    let suspended = super::live_rig::Suspended::new(pid);
    assert!(wait_until(Duration::from_secs(2), || {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|stat| {
                stat.rsplit_once(") ")
                    .map(|(_, tail)| tail.starts_with('T'))
            })
            .unwrap_or(false)
    }));
    let error = adapter
        .resolve_element_strict(&before[0], Deadline::after(5_000).unwrap())
        .err()
        .expect("stopped twin");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::AppUnresponsive);
    drop(suspended);
}
