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
    def hide_when_requested():
        if not os.path.exists(os.environ['ADL_HIDE']):
            return True
        w.destroy()
        return False
    GLib.timeout_add(20, hide_when_requested)
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
    let hide = format!("{dir}/hide-resident");
    let resident = crate::system::close::close_live_tests::rig::Orphan::spawn(
        &script,
        &[],
        &[
            ("ADL_MODE", "resident".to_owned()),
            ("ADL_HIDE", hide.clone()),
        ],
    );
    resident.wait_for_window();
    let resident_pid = resident.pid;
    std::fs::write(hide, "hide").expect("request window destruction");
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
        activate: true,
        env: BTreeMap::from([("ADL_MODE".to_owned(), "normal".to_owned())]),
        ..options(&[])
    };
    let second = launch(&script, &normal).expect("a launch of an app with no window");
    let mut owned = Owned(Vec::new());
    owned.keep(second.pid.get());
    assert!(second.window.is_some(), "a window is reported: {second:?}");
    assert_ne!(
        second.pid.get(),
        resident_pid,
        "the app that shows the window is reported, not the one with none"
    );
    assert!(!x_windows_of(second.pid.get()).is_empty());
}

fn windows_of_pid(pid: u32) -> usize {
    crate::system::list_windows(
        &SharedBus::default(),
        &agent_desktop_core::WindowFilter::default(),
        deadline(8_000),
    )
    .map(|windows| windows.iter().filter(|w| w.pid.get() == pid).count())
    .expect("a successful window read")
}

#[test]
#[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
fn live_a_forwarded_launch_without_matching_argv_does_not_claim_an_owner() {
    let mut owned = Owned(Vec::new());
    let first = launch("nautilus", &options(&[])).expect("the first launch");
    let pid = owned.keep(first.pid.get());
    wait_until("the initial native window", || x_windows_of(pid).len() == 1);
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
