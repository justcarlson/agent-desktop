use std::cell::{Cell, RefCell};

use agent_desktop_core::{
    DeliverySemantics, ErrorCode, InteractionPolicy, KeyCombo, ProcessId, ProcessIdentity,
};

use super::*;
use crate::actions::physical_target::tests::fake::{FakeWorld, check_of, deadline, not_delivered};
use crate::system::process_facts::ProcessFacts;

const PID: u32 = 4242;
const OTHER: u32 = 99;
const INSTANCE: &str = "boot:100";

struct Keys<'a> {
    physical: &'a FakeWorld,
    focused: Option<u32>,
    active: Option<u32>,
    lookup: Lookup,
    changes_after_send: bool,
    reads: RefCell<Vec<&'static str>>,
    lookups: Cell<u32>,
    stopped: bool,
}

fn facts(instance: &str) -> ProcessFacts {
    ProcessFacts {
        comm: "app".to_owned(),
        exe_basename: None,
        instance: instance.to_owned(),
        web_family: None,
    }
}

impl<'a> Keys<'a> {
    fn owning(physical: &'a FakeWorld) -> Self {
        Self {
            physical,
            focused: Some(PID),
            active: Some(PID),
            lookup: Lookup::Found(facts(INSTANCE)),
            changes_after_send: false,
            reads: RefCell::new(Vec::new()),
            lookups: Cell::new(0),
            stopped: false,
        }
    }

    fn read_count(&self, name: &str) -> usize {
        self.reads
            .borrow()
            .iter()
            .filter(|read| **read == name)
            .count()
    }
}

impl KeyWorld for Keys<'_> {
    fn stopped(&self, _pid: u32) -> bool {
        self.stopped
    }

    fn lookup(&self, _pid: u32) -> Lookup {
        self.lookups.set(self.lookups.get() + 1);
        if self.changes_after_send && self.physical.count("sent") > 0 {
            return Lookup::Found(facts("boot:other"));
        }
        self.lookup.clone()
    }

    fn focused_pid(&self, _deadline: Deadline) -> Result<Option<u32>, AdapterError> {
        self.reads.borrow_mut().push("focused");
        Ok(self.focused)
    }

    fn active_client_pid(&self, _deadline: Deadline) -> Option<u32> {
        self.reads.borrow_mut().push("active");
        self.active
    }
}

fn process() -> ProcessIdentity {
    ProcessIdentity::new(ProcessId::new(PID), INSTANCE)
}

fn combo() -> KeyCombo {
    KeyCombo {
        key: "n".to_owned(),
        modifiers: Vec::new(),
    }
}

fn press(
    physical: &FakeWorld,
    keys: &Keys<'_>,
    policy: InteractionPolicy,
) -> Result<ActionResult, AdapterError> {
    press_in(Press {
        physical,
        keys,
        process: &process(),
        combo: &combo(),
        policy,
        deadline: deadline(),
    })
}

#[test]
fn the_active_owner_gets_one_synthesis_under_headless_and_headed_policy() {
    for policy in [InteractionPolicy::headless(), InteractionPolicy::headed()] {
        let physical = FakeWorld::x11();
        let keys = Keys::owning(&physical);
        let result = press(&physical, &keys, policy).expect("the owner of the active window");
        assert_eq!(physical.count("sent"), 1);
        assert_eq!(
            result.disposition(),
            DeliverySemantics::delivered_unverified()
        );
        assert_eq!(result.steps[0].label(), "XTEST.press_key");
        assert_eq!(result.steps[0].verified(), Some(false));
    }
}

#[test]
fn a_popup_grab_in_another_app_leaves_the_frame_inactive_and_nothing_is_sent() {
    let physical = FakeWorld::x11();
    let mut keys = Keys::owning(&physical);
    keys.focused = None;
    let error = press(&physical, &keys, InteractionPolicy::headed()).unwrap_err();
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(not_delivered(&error));
    assert!(physical.sent_nothing());
}

#[test]
fn another_application_owning_either_active_measure_gets_no_keys() {
    for (focused, active) in [(Some(OTHER), Some(PID)), (Some(PID), Some(OTHER))] {
        let physical = FakeWorld::x11();
        let mut keys = Keys::owning(&physical);
        keys.focused = focused;
        keys.active = active;
        press(&physical, &keys, InteractionPolicy::headed()).unwrap_err();
        assert!(physical.sent_nothing());
    }
}

#[test]
fn a_window_with_no_known_owner_on_the_display_is_not_the_target() {
    let physical = FakeWorld::x11();
    let mut keys = Keys::owning(&physical);
    keys.active = None;
    press(&physical, &keys, InteractionPolicy::headed()).unwrap_err();
    assert!(physical.sent_nothing());
}

#[test]
fn on_wayland_the_gate_is_the_frame_alone() {
    let physical = FakeWorld::wayland();
    let mut keys = Keys::owning(&physical);
    keys.active = None;
    let result = press(&physical, &keys, InteractionPolicy::headless()).expect("frame active");
    assert_eq!(result.steps[0].label(), "RemoteDesktop.press_key");
    assert_eq!(keys.read_count("active"), 0);
    assert_eq!(physical.count("sent"), 1);

    let inactive = FakeWorld::wayland();
    let mut keys = Keys::owning(&inactive);
    keys.focused = Some(OTHER);
    press(&inactive, &keys, InteractionPolicy::headed()).unwrap_err();
    assert!(inactive.sent_nothing());
}

#[test]
fn an_inactive_wayland_press_offers_manual_focus_then_a_supported_retry() {
    let physical = FakeWorld::wayland();
    let mut keys = Keys::owning(&physical);
    keys.focused = None;
    let error = press(&physical, &keys, InteractionPolicy::headless()).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.as_ref().unwrap()["cause"], "frame_not_active");
    assert!(not_delivered(&error));
    assert!(physical.sent_nothing());
    let hint = error.suggestion.as_deref().unwrap();
    assert!(
        hint.contains("bring the target window to the front yourself"),
        "{hint}"
    );
    assert!(
        hint.contains("global press without --app, --window-id or --headed"),
        "{hint}"
    );

    crate::actions::physical_keyboard::press_key_global_steps(&physical, &combo(), deadline())
        .unwrap();
    assert_eq!(physical.count("sent"), 1);
}

#[test]
fn a_background_target_fails_by_policy_and_neither_case_calls_synthesis() {
    let allowed = FakeWorld::x11();
    let mut keys = Keys::owning(&allowed);
    keys.focused = Some(OTHER);
    keys.active = Some(OTHER);
    let error = press(&allowed, &keys, InteractionPolicy::headed()).unwrap_err();
    assert!(
        error.message.contains("Focus did not reach"),
        "{}",
        error.message
    );
    assert!(allowed.sent_nothing());

    let forbidden = FakeWorld::x11();
    let mut keys = Keys::owning(&forbidden);
    keys.focused = Some(OTHER);
    keys.active = Some(OTHER);
    let error = press(&forbidden, &keys, InteractionPolicy::headless()).unwrap_err();
    assert!(error.message.contains("background"), "{}", error.message);
    assert_eq!(check_of(&error).as_deref(), Some("window_active"));
    assert!(not_delivered(&error));
    assert!(forbidden.sent_nothing());
}

#[test]
fn a_token_that_changed_before_synthesis_is_refused_with_nothing_read_or_sent() {
    for lookup in [Lookup::Found(facts("boot:other")), Lookup::Gone] {
        let physical = FakeWorld::x11();
        let mut keys = Keys::owning(&physical);
        keys.lookup = lookup;
        let error = press(&physical, &keys, InteractionPolicy::headed()).unwrap_err();
        assert_eq!(error.code, ErrorCode::AppUnresponsive);
        assert!(not_delivered(&error));
        assert!(physical.sent_nothing());
        assert_eq!(keys.read_count("focused"), 0);
    }
}

#[test]
fn a_token_that_changed_after_synthesis_is_a_delivered_unverified_error() {
    let physical = FakeWorld::x11();
    let mut keys = Keys::owning(&physical);
    keys.changes_after_send = true;
    let error = press(&physical, &keys, InteractionPolicy::headed()).unwrap_err();
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(physical.count("sent"), 1);
}

#[test]
fn an_unreadable_process_does_not_end_the_press() {
    let physical = FakeWorld::x11();
    let mut keys = Keys::owning(&physical);
    keys.lookup = Lookup::Unreadable;
    press(&physical, &keys, InteractionPolicy::headed()).expect("unreadable is not changed");
    assert_eq!(physical.count("sent"), 1);
}

#[test]
fn a_stopped_target_is_unresponsive_and_nothing_is_synthesised() {
    let physical = FakeWorld::x11();
    let mut keys = Keys::owning(&physical);
    keys.stopped = true;
    let error = press(&physical, &keys, InteractionPolicy::headed()).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(physical.count("sent"), 0);
}
