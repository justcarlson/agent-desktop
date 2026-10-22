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
fn unknown_desktop_refuses_session_classification() {
    assert!(gnome_backend(&[], Some("unrecognized-desktop")).is_err());
    assert!(!gnome_backend(&[], None).unwrap());
    assert!(gnome_backend(&["org.gnome.Shell".into()], Some("unrecognized-desktop")).unwrap());
}
