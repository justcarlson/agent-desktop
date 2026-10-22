use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use agent_desktop_core::launch_options::LaunchOptions;

use super::*;

const RESIDENT_APP: &str = "#!/usr/bin/env python3
import os, gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, GLib
Gtk.init()
if os.environ.get('ADL_MODE') == 'none':
    Gtk.main()
w = Gtk.Window(title='adl-resident')
w.set_default_size(200, 100)
w.show_all()
if os.environ.get('ADL_MODE') == 'resident':
    GLib.timeout_add(1200, lambda: (w.destroy(), False)[1])
    GLib.timeout_add(60000, Gtk.main_quit)
else:
    w.connect('destroy', Gtk.main_quit)
Gtk.main()
";

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_running_app_with_no_window_is_started_again_and_a_window_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/adl-resident");
    std::fs::write(&script, RESIDENT_APP).expect("a script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("modes");
    let mut owned = Owned(Vec::new());
    let resident = LaunchOptions {
        env: BTreeMap::from([("ADL_MODE".to_owned(), "resident".to_owned())]),
        ..options(&[])
    };
    let first = launch(&script, &resident).expect("the first launch");
    let resident_pid = owned.keep(first.pid.get());
    let started = Instant::now();
    while started.elapsed() < SETTLE && windows_of_pid(resident_pid) > 0 {
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(windows_of_pid(resident_pid), 0, "the window is gone");
    assert!(
        process_facts::read(resident_pid).is_some(),
        "the process stays"
    );

    let normal = LaunchOptions {
        env: BTreeMap::from([("ADL_MODE".to_owned(), "normal".to_owned())]),
        ..options(&[])
    };
    let second = launch(&script, &normal).expect("a launch of an app with no window");
    owned.keep(second.pid.get());
    assert!(second.window.is_some(), "a window is reported: {second:?}");
    assert_ne!(
        second.pid.get(),
        resident_pid,
        "the app that shows the window is reported, not the one with none"
    );
}

fn windows_of_pid(pid: u32) -> usize {
    crate::system::list_windows(
        &SharedBus::default(),
        &agent_desktop_core::WindowFilter::default(),
        deadline(8_000),
    )
    .map(|windows| windows.iter().filter(|w| w.pid.get() == pid).count())
    .unwrap_or(0)
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_forwarded_launch_without_matching_argv_does_not_claim_an_owner() {
    let mut owned = Owned(Vec::new());
    let first = launch("nautilus", &options(&[])).expect("the first launch");
    let pid = owned.keep(first.pid.get());
    let before = x_windows_of(pid);
    assert_eq!(before.len(), 1);
    let refusal = launch("nautilus", &options(&["--new-window"]))
        .expect_err("different argv cannot establish the forwarded owner");
    assert_eq!(refusal.code, ErrorCode::ActionFailed);
    assert_eq!(
        refusal.disposition,
        agent_desktop_core::DeliverySemantics::delivered_unverified()
    );
    wait_until("native forwarding effect", || {
        x_windows_of(pid).len() > before.len()
    });
    let after = x_windows_of(pid);
    assert!(before.iter().all(|id| after.contains(id)));
    println!(
        "independent xprop: pid {pid} opened another window; the unproven argv handoff remains delivered_unverified"
    );
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_launch_that_shows_no_window_does_not_wait_for_the_whole_timeout() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let script = format!("{dir}/adl-resident");
    std::fs::write(&script, RESIDENT_APP).expect("a script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("modes");
    let mut owned = Owned(Vec::new());
    let options = LaunchOptions {
        env: BTreeMap::from([("ADL_MODE".to_owned(), "none".to_owned())]),
        ..options(&[])
    };
    for round in 0..2 {
        let started = Instant::now();
        let result = launch(&script, &options).expect("a launch");
        owned.keep(result.pid.get());
        assert!(result.window.is_none(), "round {round}: {result:?}");
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "round {round} waited {:?} of a 15 s timeout",
            started.elapsed()
        );
    }
}
