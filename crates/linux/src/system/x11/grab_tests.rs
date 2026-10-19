use std::time::Duration;

use agent_desktop_core::Deadline;

use super::*;
use crate::system::display_server::{Cause, Unavailable};

fn deadline(ms: u64) -> Deadline {
    Deadline::from_duration(Duration::from_millis(ms)).unwrap()
}

#[test]
fn a_closure_result_passes_through_unchanged() {
    assert_eq!(bounded_grab(deadline(2_000), || Ok(7)), Ok(7));
}

#[test]
fn unreachable_denied_and_a_missed_deadline_are_three_distinct_errors() {
    let unreachable = bounded_grab::<()>(deadline(2_000), || {
        Err(GrabError::Fault(MonitorFault::Unreachable(":9".to_owned())))
    });
    let denied = bounded_grab::<()>(deadline(2_000), || {
        Err(GrabError::Fault(MonitorFault::Denied("no".to_owned())))
    });
    let late = bounded_grab::<()>(deadline(30), || {
        std::thread::sleep(Duration::from_millis(600));
        Ok(())
    });
    assert_eq!(
        unreachable,
        Err(GrabError::Fault(MonitorFault::Unreachable(":9".to_owned())))
    );
    assert_eq!(
        denied,
        Err(GrabError::Fault(MonitorFault::Denied("no".to_owned())))
    );
    assert_eq!(late, Err(GrabError::Timeout));
}

#[test]
fn only_an_x11_session_names_a_display() {
    assert_eq!(
        display_of(DisplayServer::X11(":5".to_owned())),
        Ok(":5".to_owned())
    );
    assert!(matches!(
        display_of(DisplayServer::Wayland),
        Err(GrabError::Fault(MonitorFault::Unreachable(_)))
    ));
    assert!(matches!(
        display_of(DisplayServer::Unavailable(Unavailable {
            cause: Cause::Missing,
            detail: String::new(),
        })),
        Err(GrabError::Fault(MonitorFault::Unreachable(_)))
    ));
}
