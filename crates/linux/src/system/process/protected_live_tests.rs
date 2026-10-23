use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode};

use super::is_protected;
use super::session_origin::LAUNCH_MARKER;
use crate::system::SharedBus;
use crate::system::close::close_app;
use crate::system::close::close_live_tests::rig::{Orphan, lease};
use crate::system::close::close_live_tests::{listed, portal_backend};

const PLAIN: &str = "org.freedesktop.impl.portal.desktop.PlainFixture";
const LAUNCHED: &str = "org.freedesktop.impl.portal.desktop.LaunchedFixture";
const HUNG: &str = "org.freedesktop.impl.portal.desktop.HungFixture";
const HUNG_REFUSED: &str = "org.freedesktop.impl.portal.desktop.HungRefusedFixture";

fn protected(orphan: &Orphan) -> bool {
    let deadline = Deadline::after(10_000).expect("a deadline builds");
    is_protected(&SharedBus::default(), orphan.pid, &orphan.token, deadline)
        .expect("the ownership of the process is readable")
}

#[test]
#[ignore = "starts owned portal backends on a private session bus"]
fn a_silent_process_that_owns_a_service_name_is_protected_unless_a_launch_of_this_tool_started_it()
{
    let plain = portal_backend(PLAIN, &[]);
    let launched = portal_backend(LAUNCHED, &[(LAUNCH_MARKER, "1".to_owned())]);
    assert!(
        protected(&plain),
        "a process the session could have started for its name is protected"
    );
    assert!(
        !protected(&launched),
        "a process a launch of this tool started is never protected by a name it owns"
    );
    println!(
        "independent D-Bus credentials: pid {} and pid {} own a service name each; only the one without the launch marker is protected",
        plain.pid, launched.pid
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn a_hung_application_with_a_window_is_not_protected_and_a_hung_windowless_name_owner_is() {
    let bus = SharedBus::default();
    let editor = Orphan::spawn("gnome-text-editor", &["--new-window"], &[]);
    listed(&bus, editor.pid);
    let backend = portal_backend(HUNG, &[]);
    editor.stop();
    backend.stop();
    let (editor_protected, backend_protected) = (protected(&editor), protected(&backend));
    println!(
        "independent /proc state and xprop windows: stopped editor pid {} owns a window and a name, stopped backend pid {} owns a name only",
        editor.pid, backend.pid
    );
    assert!(
        !editor_protected,
        "a hung application the window manager shows a window of is not a session service"
    );
    assert!(
        backend_protected,
        "a hung process with a session-bus name and no window stays protected"
    );
}

#[test]
#[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
fn a_hung_application_with_a_window_is_force_closed_and_a_hung_windowless_name_owner_is_refused() {
    let bus = SharedBus::default();
    let editor = Orphan::spawn("gnome-text-editor", &["--new-window"], &[]);
    let app = listed(&bus, editor.pid);
    editor.stop();
    let graceful = close_app(&bus, &app, false, &lease(10_000)).expect_err("stopped");
    assert_eq!(graceful.code, ErrorCode::AppUnresponsive);
    assert!(editor.alive(), "a graceful close signals nothing");
    close_app(&bus, &app, true, &lease(10_000)).expect("a forced close ends a hung application");
    assert!(!editor.alive());

    let backend = portal_backend(HUNG_REFUSED, &[]);
    backend.stop();
    for force in [false, true] {
        let error = close_app(
            &bus,
            &backend.app("ordinary application"),
            force,
            &lease(5_000),
        )
        .expect_err("protected");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert!(backend.alive(), "force={force}");
    }
}
