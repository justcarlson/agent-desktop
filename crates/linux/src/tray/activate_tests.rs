use std::cell::RefCell;

use agent_desktop_core::{
    Action, ActionRequest, AdapterError, Deadline, DeliveryDisposition, ErrorCode,
    InteractionLease, InteractionPolicy, MouseButton, Point,
};

use super::super::world::ShellWorld;
use super::*;
use crate::system::shell::container::Landmark;
use crate::tree::AtspiElement;

#[derive(Default)]
struct Wire {
    present: bool,
    sent: RefCell<Vec<String>>,
}

impl ItemWire for Wire {
    fn owner_present(&self) -> Result<bool, AdapterError> {
        Ok(self.present)
    }

    fn send(&self, method: &str) -> Result<(), AdapterError> {
        self.sent.borrow_mut().push(method.to_owned());
        Ok(())
    }
}

#[derive(Default)]
struct World {
    clicks: RefCell<Vec<&'static str>>,
}

impl ShellWorld for World {
    fn locate(&self, _: Landmark, _: Deadline) -> Result<AtspiElement, AdapterError> {
        Ok(AtspiElement::new(":1.1", "/panel", 1))
    }

    fn click(
        &self,
        _: &AtspiElement,
        button: MouseButton,
        _: InteractionPolicy,
        _: &InteractionLease,
    ) -> Result<Point, AdapterError> {
        self.clicks.borrow_mut().push(match button {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        });
        Ok(Point { x: 1.0, y: 1.0 })
    }

    fn panel_node(&self, _: &str, _: Deadline) -> Result<AtspiElement, AdapterError> {
        Ok(AtspiElement::new(":1.1", "/node", 1))
    }

    fn panel_extents(&self, _: &str, _: Deadline) -> Option<agent_desktop_core::Rect> {
        None
    }
}

fn methods(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn run_with(
    methods: &[String],
    request: ActionRequest,
    wire: &Wire,
    world: &World,
) -> Result<agent_desktop_core::ActionResult, AdapterError> {
    let lease = InteractionLease::guarded(Deadline::after(1_000).expect("deadline"), ())
        .expect("a test lease builds");
    let item = Item {
        title: "Dropbox",
        methods,
        request: &request,
        deadline: Deadline::after(1_000).expect("deadline"),
    };
    run(&item, wire, world, &lease)
}

fn present() -> Wire {
    Wire {
        present: true,
        ..Wire::default()
    }
}

#[test]
fn a_listed_activate_is_sent_for_a_click_even_when_headed() {
    for request in [
        ActionRequest::headless(Action::Click),
        ActionRequest::headed(Action::Click),
    ] {
        let (wire, world) = (present(), World::default());
        run_with(
            &methods(&["Activate", "SecondaryActivate"]),
            request,
            &wire,
            &world,
        )
        .expect("delivered");
        assert_eq!(*wire.sent.borrow(), ["Activate"]);
        assert!(world.clicks.borrow().is_empty());
    }
}

#[test]
fn a_headless_click_with_no_activate_is_refused_and_nothing_is_sent() {
    for listed in [
        methods(&[]),
        methods(&["SecondaryActivate"]),
        methods(&["XAyatanaSecondaryActivate"]),
        methods(&["SecondaryActivate", "XAyatanaSecondaryActivate", "Scroll"]),
    ] {
        let (wire, world) = (present(), World::default());
        let error = run_with(
            &listed,
            ActionRequest::headless(Action::Click),
            &wire,
            &world,
        )
        .expect_err("refused");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(
            error.details.expect("details")["physical_delivery_started"],
            false
        );
        assert!(error.suggestion.expect("suggestion").contains("--headed"));
        assert_eq!(
            error.disposition.delivery(),
            DeliveryDisposition::NotDelivered
        );
        assert!(wire.sent.borrow().is_empty());
        assert!(world.clicks.borrow().is_empty());
    }
}

#[test]
fn a_headed_click_with_no_activate_uses_the_panel_pointer() {
    let (wire, world) = (present(), World::default());
    run_with(
        &methods(&["SecondaryActivate"]),
        ActionRequest::headed(Action::Click),
        &wire,
        &world,
    )
    .expect("delivered");
    assert_eq!(*world.clicks.borrow(), ["left"]);
    assert!(wire.sent.borrow().is_empty());
}

#[test]
fn right_click_uses_context_menu_when_listed_and_the_right_button_otherwise() {
    let (wire, world) = (present(), World::default());
    run_with(
        &methods(&[
            "ContextMenu",
            "SecondaryActivate",
            "XAyatanaSecondaryActivate",
        ]),
        ActionRequest::headless(Action::RightClick),
        &wire,
        &world,
    )
    .expect("delivered");
    assert_eq!(*wire.sent.borrow(), ["ContextMenu"]);

    let (wire, world) = (present(), World::default());
    run_with(
        &methods(&[]),
        ActionRequest::headed(Action::RightClick),
        &wire,
        &world,
    )
    .expect("delivered");
    assert_eq!(*world.clicks.borrow(), ["right"]);

    let (wire, world) = (present(), World::default());
    assert!(
        run_with(
            &methods(&[]),
            ActionRequest::headless(Action::RightClick),
            &wire,
            &world
        )
        .is_err()
    );
}

#[test]
fn a_missing_owner_fails_closed_before_any_call() {
    let (wire, world) = (Wire::default(), World::default());
    let error = run_with(
        &methods(&["Activate"]),
        ActionRequest::headless(Action::Click),
        &wire,
        &world,
    )
    .expect_err("owner gone");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert!(wire.sent.borrow().is_empty());
}

#[test]
fn a_delivered_activate_is_reported_unverified() {
    let (wire, world) = (present(), World::default());
    let result = run_with(
        &methods(&["Activate"]),
        ActionRequest::headless(Action::Click),
        &wire,
        &world,
    )
    .expect("delivered");
    assert_eq!(
        result.disposition().delivery(),
        DeliveryDisposition::DeliveredUnverified
    );
}

#[test]
fn an_action_other_than_click_is_not_supported_on_an_item() {
    let (wire, world) = (present(), World::default());
    let error = run_with(
        &methods(&["Activate"]),
        ActionRequest::headless(Action::Toggle),
        &wire,
        &world,
    )
    .expect_err("unsupported");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert!(wire.sent.borrow().is_empty());
}
