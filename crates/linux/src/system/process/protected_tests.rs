use super::*;
use std::os::unix::net::UnixListener;

#[test]
fn socket_ownership_belongs_to_the_listener_process_not_its_name() {
    let path = std::env::temp_dir().join(format!("ad-owner-{}.sock", std::process::id()));
    let listener = UnixListener::bind(&path).unwrap();
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let own = owns_socket(std::process::id(), &path);
    let unrelated = owns_socket(child.id(), &path);
    child.kill().unwrap();
    child.wait().unwrap();
    drop(listener);
    std::fs::remove_file(path).unwrap();
    assert!(own.unwrap());
    assert!(!unrelated.unwrap());
}

fn no() -> Result<bool, AdapterError> {
    Ok(false)
}

fn yes() -> Result<bool, AdapterError> {
    Ok(true)
}

#[test]
fn a_silent_process_is_protected_by_a_session_bus_name_unless_a_launch_started_it() {
    assert!(silent_app_is_session_service(no, yes).unwrap());
    assert!(!silent_app_is_session_service(no, no).unwrap());
    assert!(!silent_app_is_session_service(yes, yes).unwrap());
}

#[test]
fn a_launched_silent_process_is_cleared_without_reading_the_session_bus() {
    let reads = std::cell::Cell::new(0);
    let named = || {
        reads.set(reads.get() + 1);
        yes()
    };
    assert!(!silent_app_is_session_service(yes, named).unwrap());
    assert_eq!(reads.get(), 0);
}

#[test]
fn a_launch_marker_that_cannot_be_read_refuses_instead_of_protecting_by_name() {
    let unreadable = || Err(unknown());
    let error = silent_app_is_session_service(unreadable, yes).expect_err("unproven origin");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn an_unreachable_display_is_reported_as_the_display_not_as_unproven_ownership() {
    use crate::system::display::display_cause::Cause;
    use crate::system::display::display_unavailable::Unavailable;
    let error = socket_of(DisplayServer::Unavailable(Unavailable {
        cause: Cause::Missing,
        detail: "DISPLAY=:999 names a server that is not running".to_owned(),
    }))
    .expect_err("no socket without a display");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(error.message.contains("display server is not running"));
}
