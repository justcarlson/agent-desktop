use std::os::unix::net::UnixListener;

use super::{SocketProbe, local_number};
use crate::system::display_server::{Probe, Reach};

#[test]
fn only_local_display_names_have_a_number() {
    assert_eq!(local_number(":1"), Some("1"));
    assert_eq!(local_number(":12.0"), Some("12"));
    assert_eq!(local_number("unix:3"), Some("3"));
    assert_eq!(local_number("host:0"), None);
    assert_eq!(local_number(":"), None);
    assert_eq!(local_number(":x"), None);
}

#[test]
fn a_remote_display_is_assumed_reachable_because_it_is_not_probed() {
    assert_eq!(SocketProbe.x11("host:0"), Reach::Connected);
}

#[test]
fn a_listening_socket_connects_a_stale_file_is_refused_and_an_absent_one_is_missing() {
    let dir = std::env::temp_dir().join(format!("ad-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let live = dir.join("live");
    let listener = UnixListener::bind(&live).expect("bind");
    assert_eq!(SocketProbe.wayland(&live), Reach::Connected);
    drop(listener);
    assert_eq!(SocketProbe.wayland(&live), Reach::Refused);
    assert_eq!(SocketProbe.wayland(&dir.join("absent")), Reach::Missing);
    std::fs::remove_dir_all(&dir).expect("cleanup");
}
