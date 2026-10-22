use agent_desktop_core::Deadline;

use super::is_protected;
use super::session_origin::LAUNCH_MARKER;
use crate::system::SharedBus;
use crate::system::close::close_live_tests::portal_backend;
use crate::system::close::close_live_tests::rig::Orphan;

const PLAIN: &str = "org.freedesktop.impl.portal.desktop.PlainFixture";
const LAUNCHED: &str = "org.freedesktop.impl.portal.desktop.LaunchedFixture";

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
