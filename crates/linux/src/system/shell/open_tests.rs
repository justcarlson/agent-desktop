use std::cell::{Cell, RefCell};
use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, InteractionPolicy, WindowInfo, WindowState,
};

use crate::system::shell::container::Located;
use crate::system::shell::kinds::ShellKind;
use crate::system::shell::light::Light;
use crate::system::shell::open::{Opened, Surface, close_with, open_with, showing_of};
use crate::tree::AtspiElement;

struct World {
    showing: Cell<bool>,
    opens_on_press: bool,
    closes_on_press: bool,
    window_fails: bool,
    press_cleanup_fails: bool,
    events: RefCell<Vec<&'static str>>,
}

impl World {
    fn closed() -> Self {
        Self {
            showing: Cell::new(false),
            opens_on_press: true,
            closes_on_press: true,
            window_fails: false,
            press_cleanup_fails: false,
            events: RefCell::default(),
        }
    }

    fn open() -> Self {
        let world = Self::closed();
        world.showing.set(true);
        world
    }

    fn events(&self) -> Vec<&'static str> {
        self.events.borrow().clone()
    }
}

impl Surface for World {
    fn showing(&self, _kind: ShellKind, _deadline: Deadline) -> Result<bool, AdapterError> {
        Ok(self.showing.get())
    }

    fn press_opener(
        &self,
        _kind: ShellKind,
        _policy: InteractionPolicy,
    ) -> Result<(), AdapterError> {
        self.events.borrow_mut().push("press");
        if self.showing.get() && self.closes_on_press {
            self.showing.set(false);
        } else if !self.showing.get() && self.opens_on_press {
            self.showing.set(true);
        }
        if self.press_cleanup_fails {
            return Err(AdapterError::new(ErrorCode::ActionNotSupported, "Stop refused")
                .with_details(serde_json::json!({ "physical_delivery_started": true }))
                .with_disposition(agent_desktop_core::DeliverySemantics::delivered_unverified()));
        }
        Ok(())
    }

    fn window(&self, _kind: ShellKind, _deadline: Deadline) -> Result<WindowInfo, AdapterError> {
        if self.window_fails {
            return Err(AdapterError::timeout("window info could not be read"));
        }
        Ok(WindowInfo {
            id: "w-1-2-3".to_owned(),
            title: "t".to_owned(),
            app: "gnome-shell".to_owned(),
            pid: 1.into(),
            process_instance: None,
            bounds: None,
            state: WindowState::default(),
        })
    }

    fn pause(&self, duration: Duration) {
        std::thread::sleep(duration.min(Duration::from_millis(5)));
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("a deadline builds")
}

#[test]
fn a_surface_already_showing_is_adopted_with_no_event() {
    let world = World::open();
    let (_, how) = open_with(
        &world,
        ShellKind::MessageList,
        InteractionPolicy::headed(),
        deadline(2_000),
    )
    .expect("an open surface is adopted");
    assert_eq!(how, Opened::Adopted);
    assert!(world.events().is_empty());
}

#[test]
fn a_failed_window_read_of_an_adopted_surface_preserves_nonownership() {
    let mut world = World::open();
    world.window_fails = true;
    let error = open_with(
        &world,
        ShellKind::MessageList,
        InteractionPolicy::headed(),
        deadline(2_000),
    )
    .expect_err("window info failed");
    assert_eq!(
        error.details.expect("details")["surface_opened_by_call"],
        false
    );
    assert!(world.events().is_empty());
}

#[test]
fn a_failed_window_read_after_opening_preserves_owned_state() {
    let mut world = World::closed();
    world.window_fails = true;
    let error = open_with(
        &world,
        ShellKind::MessageList,
        InteractionPolicy::headed(),
        deadline(2_000),
    )
    .expect_err("window info failed");
    assert_eq!(
        error.details.expect("details")["surface_opened_by_call"],
        true
    );
    assert_eq!(world.events(), ["press"]);
}

#[test]
fn a_delivered_opener_error_preserves_ownership_for_restoration() {
    let mut world = World::closed();
    world.press_cleanup_fails = true;
    let error = open_with(
        &world,
        ShellKind::MessageList,
        InteractionPolicy::headed(),
        deadline(2_000),
    )
    .expect_err("the click arrived but its pointer session did not stop");

    assert!(world.showing.get());
    assert_eq!(
        error.details.expect("opener progress")["surface_opened_by_call"],
        true
    );
    assert_eq!(world.events(), ["press"]);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::delivered_unverified()
    );
}

#[test]
fn a_closed_surface_is_opened_by_one_press_and_left_open() {
    let world = World::closed();
    let (_, how) = open_with(
        &world,
        ShellKind::SystemMenu,
        InteractionPolicy::headed(),
        deadline(2_000),
    )
    .expect("the press opens it");
    assert_eq!(how, Opened::Opened);
    assert_eq!(world.events(), ["press"]);
    assert!(world.showing.get(), "the opened surface stays open");
}

#[test]
fn a_policy_that_is_not_headed_is_refused_before_the_first_event() {
    for policy in [
        InteractionPolicy::headless(),
        InteractionPolicy::focus_fallback(),
    ] {
        let world = World::closed();
        let error = open_with(&world, ShellKind::MessageList, policy, deadline(2_000))
            .expect_err("a click needs the cursor");
        assert_eq!(error.code, ErrorCode::PolicyDenied);
        assert!(world.events().is_empty());
    }
}

#[test]
fn an_open_that_never_shows_is_a_timeout_after_the_click_was_sent() {
    let mut world = World::closed();
    world.opens_on_press = false;
    let error = open_with(
        &world,
        ShellKind::MessageList,
        InteractionPolicy::headed(),
        deadline(60),
    )
    .expect_err("the surface never shows");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(
        error.details.expect("details")["physical_delivery_started"],
        true
    );
    assert_eq!(world.events(), ["press"]);
}

#[test]
fn the_dock_and_the_tray_need_no_opener_even_headless() {
    for kind in [ShellKind::Dock, ShellKind::Tray] {
        let world = World::closed();
        let (_, how) = open_with(&world, kind, InteractionPolicy::headless(), deadline(2_000))
            .expect("nothing to open");
        assert_eq!(how, Opened::Adopted);
        assert!(world.events().is_empty());
    }
}

#[test]
fn close_presses_only_a_showing_surface_and_waits_for_it_to_hide() {
    let world = World::open();
    close_with(&world, ShellKind::MessageList, deadline(2_000)).expect("it closes");
    assert_eq!(world.events(), ["press"]);
    assert!(!world.showing.get());
    close_with(&world, ShellKind::MessageList, deadline(2_000)).expect("a closed one stays");
    assert_eq!(world.events(), ["press"]);
}

#[test]
fn a_close_that_does_not_take_effect_is_an_error() {
    let mut world = World::open();
    world.closes_on_press = false;
    let error =
        close_with(&world, ShellKind::MessageList, deadline(60)).expect_err("it stays open");
    assert_eq!(error.code, ErrorCode::Timeout);
}

fn light(showing: bool) -> Light {
    Light {
        element: AtspiElement::new(":1.2", "/n", 7),
        role: 43,
        name: String::new(),
        extents: None,
        showing,
        visible: true,
        children: Vec::new(),
    }
}

#[test]
fn an_empty_open_message_list_is_showing_though_its_clear_all_button_is_hidden() {
    let located = Located {
        anchor: light(false),
        container: light(true),
    };
    assert!(showing_of(ShellKind::MessageList, &located));
}

#[test]
fn a_closed_message_list_is_not_showing_though_its_anchor_claims_to() {
    let located = Located {
        anchor: light(true),
        container: light(false),
    };
    assert!(!showing_of(ShellKind::MessageList, &located));
}

#[test]
fn the_system_menu_is_still_told_by_its_anchor() {
    let located = Located {
        anchor: light(true),
        container: light(false),
    };
    assert!(showing_of(ShellKind::SystemMenu, &located));
}
