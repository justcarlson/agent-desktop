use agent_desktop_core::{Deadline, ErrorCode};

use super::key_session::KeySession;
use super::live_shell::Shell;
use super::remote_desktop::RemoteDesktop;
use crate::system::SharedBus;
use crate::system::session_bus::SessionBus;
use crate::tree::live_launch::Launched;

fn deadline() -> Deadline {
    Deadline::after(20_000).expect("a deadline builds")
}

#[test]
#[ignore = "needs a private Wayland session with a headless gnome-shell; run through scripts/private-session.sh"]
fn remote_desktop_answers_on_the_session_bus_and_not_on_the_accessibility_bus() {
    let _launched = Launched::new();
    let _shell = Shell::start();
    let mut desktop = RemoteDesktop::open(deadline()).expect("the session bus opens");
    desktop.create(deadline()).expect("CreateSession succeeds");
    desktop.start(deadline()).expect("Start succeeds");
    desktop.stop(deadline()).expect("Stop succeeds");

    let accessibility = SharedBus::default()
        .get(deadline())
        .expect("the accessibility bus opens");
    let mut wrong = RemoteDesktop::over(accessibility.inner().clone());
    let error = wrong
        .create(deadline())
        .expect_err("the accessibility bus has no RemoteDesktop");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(SessionBus::get(deadline()).is_ok());
}
