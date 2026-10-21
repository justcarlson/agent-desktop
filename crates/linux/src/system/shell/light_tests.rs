use crate::system::shell::fake_tree::{SENTINEL, at};
use crate::system::shell::light::{Light, is_placed};
use crate::tree::AtspiElement;

fn light(extents: Option<crate::tree::Extents>) -> Light {
    Light {
        element: AtspiElement::new(":1.1", "/a", 1),
        role: 43,
        name: "Go".to_owned(),
        extents,
        showing: true,
        visible: true,
        children: Vec::new(),
    }
}

#[test]
fn extents_name_a_place_only_with_area_and_no_sentinel() {
    assert!(is_placed(at(0, 0, 72, 32)));
    assert!(is_placed(at(-10, -1, 60, 84)));
    assert!(!is_placed(at(0, 0, 0, 32)));
    assert!(!is_placed(at(0, 0, 10, 0)));
    assert!(!is_placed(SENTINEL));
}

#[test]
fn a_light_matches_by_role_and_exact_name() {
    let node = light(None);
    assert!(node.is(43, "Go"));
    assert!(!node.is(43, "go"));
    assert!(!node.is(62, "Go"));
    assert!(node.placed().is_none());
    assert_eq!(light(Some(at(1, 2, 3, 4))).placed(), Some(at(1, 2, 3, 4)));
}

#[test]
#[ignore = "owns a private bus; run inside scripts/private-session.sh --wm"]
fn shell_live_lost_role_connection_is_not_a_destroyed_node() {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};

    use agent_desktop_core::{Deadline, ErrorCode};

    struct Daemon(Child);
    impl Drop for Daemon {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let runtime = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session");
    let mut daemon = Daemon(
        Command::new("dbus-daemon")
            .args([
                "--session",
                "--nofork",
                "--print-address=1",
                &format!("--address=unix:path={runtime}/role-loss-bus"),
            ])
            .stdout(Stdio::piped())
            .spawn()
            .expect("owned bus"),
    );
    let mut address = String::new();
    BufReader::new(daemon.0.stdout.take().expect("address pipe"))
        .read_line(&mut address)
        .expect("bus readiness");
    let connection = zbus::block_on(async {
        zbus::connection::Builder::address(address.trim())
            .expect("address")
            .build()
            .await
            .expect("connection")
    });
    let element = AtspiElement::new("org.freedesktop.DBus", "/absent", 0);
    let read = || {
        crate::system::within(
            Deadline::after(2_000).expect("deadline"),
            super::role_of(&connection, &element),
        )
    };
    assert!(
        read()
            .expect("a real missing object stays absent")
            .is_none()
    );
    daemon.0.kill().expect("kill owned bus");
    assert!(!daemon.0.wait().expect("process exit oracle").success());
    let error = read().expect_err("lost role transport cannot mean a destroyed node");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(!error.permits_retry_by_default());
    println!("owned daemon exited: required shell role = {error:?}");
}
