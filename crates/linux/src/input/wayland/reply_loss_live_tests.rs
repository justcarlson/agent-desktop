use agent_desktop_core::{DeliverySemantics, RetryDisposition};

use super::*;
use crate::input::wayland::{kept_session, keysym_send::run_chunks};

struct LostReply(RemoteDesktop);

impl KeySession for LostReply {
    fn create(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.0.create(deadline)
    }

    fn start(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.0.start(deadline)
    }

    fn keysym(&mut self, keysym: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        self.0.keysym(keysym, down, deadline)?;
        Err(AdapterError::new(
            ErrorCode::ActionFailed,
            "The test discarded the compositor acknowledgement",
        ))
    }

    fn stop(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.0.stop(deadline)
    }

    fn resumed(&self) -> bool {
        self.0.resumed()
    }
}

#[test]
#[ignore = "requires scripts/private-session.sh and a private headless Wayland compositor"]
fn received_wayland_key_with_discarded_reply_is_uncertain() {
    let _guard = Launched::new();
    let shell = Shell::start();
    let mut launched = Launched::new();
    let log = format!(
        "{}/reply-loss.log",
        std::env::var("XDG_RUNTIME_DIR").expect("a private runtime dir")
    );
    let pid = launch_with(&mut launched, &log, &shell.socket);
    close_overview();
    send(pid, &keys("x")).expect("prime the kept session");
    wait_for(pid, |now| now.text == "x").expect("the entry received the primer");

    let mut session = LostReply(kept_session::take().expect("the session was retained"));
    let steps = keys("a");
    let error = run_chunks(&mut session, &[&steps], &mut || Ok(()), deadline())
        .expect_err("the acknowledgement is discarded after delivery");
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    assert_eq!(error.disposition.retry(), RetryDisposition::Unsafe);
    let details = error.details.expect("delivery evidence");
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(details["delivered_events"], 0);
    wait_for(pid, |now| now.text == "xa").expect("the unacknowledged key reached the real entry");
}
