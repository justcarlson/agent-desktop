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
