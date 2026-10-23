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

fn unwindowed() -> bool {
    false
}

fn windowed() -> bool {
    true
}

#[test]
fn a_silent_process_that_owns_a_session_bus_name_and_no_window_is_protected() {
    assert!(silent_app_is_session_service(no, unwindowed, yes).unwrap());
    assert!(!silent_app_is_session_service(no, unwindowed, no).unwrap());
}

#[test]
fn a_silent_process_with_a_managed_window_is_a_hung_application_not_a_session_service() {
    assert!(!silent_app_is_session_service(no, windowed, yes).unwrap());
    assert!(!silent_app_is_session_service(no, windowed, no).unwrap());
}

#[test]
fn a_launch_of_this_tool_is_never_protected_by_a_name_or_a_window() {
    assert!(!silent_app_is_session_service(yes, unwindowed, yes).unwrap());
    assert!(!silent_app_is_session_service(yes, windowed, yes).unwrap());
}

#[test]
fn a_window_clears_a_silent_process_without_reading_the_session_bus() {
    let reads = std::cell::Cell::new(0);
    let named = || {
        reads.set(reads.get() + 1);
        yes()
    };
    assert!(!silent_app_is_session_service(no, windowed, named).unwrap());
    assert_eq!(reads.get(), 0);
}

#[test]
fn a_launched_silent_process_is_cleared_without_reading_the_window_or_the_session_bus() {
    let reads = std::cell::Cell::new(0);
    let counted = || {
        reads.set(reads.get() + 1);
        true
    };
    let named = || {
        reads.set(reads.get() + 1);
        yes()
    };
    assert!(!silent_app_is_session_service(yes, counted, named).unwrap());
    assert_eq!(reads.get(), 0);
}

#[test]
fn the_measured_session_processes_that_do_not_answer_keep_their_protection_by_their_windows() {
    let measured = [
        (
            "activated terminal server with a window",
            false,
            true,
            true,
            false,
        ),
        (
            "activated text editor with a window",
            false,
            true,
            true,
            false,
        ),
        (
            "text editor spawned by a launch of this tool",
            true,
            true,
            true,
            false,
        ),
        (
            "activated portal backend, no window",
            false,
            false,
            true,
            true,
        ),
        (
            "activated portal frontend, no window",
            false,
            false,
            true,
            true,
        ),
    ];
    for (label, launched, has_window, owns_name, protected) in measured {
        let verdict =
            silent_app_is_session_service(|| Ok(launched), || has_window, || Ok(owns_name))
                .unwrap();
        assert_eq!(verdict, protected, "{label}");
    }
}

#[test]
fn a_launch_marker_that_cannot_be_read_refuses_instead_of_protecting_by_name() {
    let unreadable = || Err(unknown());
    let error =
        silent_app_is_session_service(unreadable, windowed, yes).expect_err("unproven origin");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_session_bus_that_cannot_be_read_refuses_a_windowless_process() {
    let unreadable = || Err(unknown());
    let error =
        silent_app_is_session_service(no, unwindowed, unreadable).expect_err("unproven ownership");
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
