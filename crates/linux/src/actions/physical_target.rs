use std::time::Duration;

use agent_desktop_core::{
    Action, ActionRequest, ActionResult, AdapterError, Deadline, DeliverySemantics, ErrorCode,
    InteractionLease, InteractionPolicy, KeyCombo, MouseButton, Point, Rect,
};
use serde_json::json;

use super::physical_click::{self, ClickSpec};
use super::physical_keyboard;
use super::physical_scroll;
use crate::input::backend::Backend;
use crate::input::delivery::{Delivery, Gate};
use crate::input::keymap::Keymap;
use crate::input::keysym_step::KeysymStep;
use crate::input::plan::Plan;
use crate::system::SharedBus;
use crate::tree::AtspiElement;

/// Everything a physical leg reads from or sends to the desktop, so each leg
/// runs against a fixed desktop in a test.
pub(crate) trait PhysicalWorld {
    fn backend(&self) -> Result<Backend, AdapterError>;
    fn keymap(&self, deadline: Deadline) -> Result<Keymap, AdapterError>;
    fn frame_active(
        &self,
        element: &AtspiElement,
        deadline: Deadline,
    ) -> Result<Option<bool>, AdapterError>;
    fn bounds(
        &self,
        element: &AtspiElement,
        deadline: Deadline,
    ) -> Result<Option<Rect>, AdapterError>;
    fn plan_combo(&self, map: &Keymap, combo: &KeyCombo) -> Result<Plan, AdapterError>;
    fn plan_keysyms(&self, combo: &KeyCombo) -> Result<Vec<KeysymStep>, AdapterError>;
    fn plan_text(&self, map: &Keymap, text: &str) -> Result<Vec<Plan>, AdapterError>;
    fn plan_text_keysyms(&self, text: &str) -> Vec<Vec<KeysymStep>>;
    fn plan_click(
        &self,
        map: &Keymap,
        at: Point,
        button: &MouseButton,
        count: u32,
    ) -> Result<Plan, AdapterError>;
    fn plan_wheel(
        &self,
        map: &Keymap,
        at: Point,
        deltas: (f64, f64),
    ) -> Result<Vec<Plan>, AdapterError>;
    fn send_plans(
        &self,
        plans: Vec<Plan>,
        gate: Gate<'_>,
        deadline: Deadline,
    ) -> Result<Delivery, AdapterError>;
    fn send_keysyms(
        &self,
        chunks: &[Vec<KeysymStep>],
        gate: Gate<'_>,
        deadline: Deadline,
    ) -> Result<Delivery, AdapterError>;
    fn pointer_refusal(&self, what: &'static str) -> AdapterError;
    fn pause(&self, duration: Duration);
}

pub(crate) struct LiveWorld<'a> {
    pub(crate) bus: &'a SharedBus,
    pub(crate) lease: &'a InteractionLease,
}

impl PhysicalWorld for LiveWorld<'_> {
    fn backend(&self) -> Result<Backend, AdapterError> {
        crate::input::backend::backend()
    }

    fn keymap(&self, deadline: Deadline) -> Result<Keymap, AdapterError> {
        match self.backend()? {
            Backend::X11(display) => crate::input::x11::session::live_keymap(&display, deadline),
            Backend::Wayland => Err(self.pointer_refusal("keymap")),
        }
    }

    fn frame_active(
        &self,
        element: &AtspiElement,
        deadline: Deadline,
    ) -> Result<Option<bool>, AdapterError> {
        let Some(frame) = element.frame() else {
            return Ok(None);
        };
        super::frame_active::frame_active(self.bus, frame, deadline)
    }

    fn bounds(
        &self,
        element: &AtspiElement,
        deadline: Deadline,
    ) -> Result<Option<Rect>, AdapterError> {
        let handle = element.clone().into_native_handle();
        crate::tree::get_element_bounds(self.bus, &handle, deadline)
    }

    fn plan_combo(&self, map: &Keymap, combo: &KeyCombo) -> Result<Plan, AdapterError> {
        crate::input::key_plan::plan_combo(map, combo)
    }

    fn plan_keysyms(&self, combo: &KeyCombo) -> Result<Vec<KeysymStep>, AdapterError> {
        crate::input::key_plan::plan_keysyms(combo)
    }

    fn plan_text(&self, map: &Keymap, text: &str) -> Result<Vec<Plan>, AdapterError> {
        crate::input::text_plan::plan_text(map, text)
    }

    fn plan_text_keysyms(&self, text: &str) -> Vec<Vec<KeysymStep>> {
        crate::input::text_plan::plan_text_keysyms(text)
    }

    fn plan_click(
        &self,
        map: &Keymap,
        at: Point,
        button: &MouseButton,
        count: u32,
    ) -> Result<Plan, AdapterError> {
        crate::input::pointer_plan::plan_click(map, at, button, count, &[])
    }

    fn plan_wheel(
        &self,
        map: &Keymap,
        at: Point,
        deltas: (f64, f64),
    ) -> Result<Vec<Plan>, AdapterError> {
        crate::input::wheel_plan::plan_wheel(map, at, deltas.0, deltas.1, &[])
    }

    fn send_plans(
        &self,
        plans: Vec<Plan>,
        gate: Gate<'_>,
        _deadline: Deadline,
    ) -> Result<Delivery, AdapterError> {
        crate::input::deliver::send_plans(plans, gate, self.lease)
    }

    fn send_keysyms(
        &self,
        chunks: &[Vec<KeysymStep>],
        gate: Gate<'_>,
        deadline: Deadline,
    ) -> Result<Delivery, AdapterError> {
        crate::input::deliver::send_keysyms(chunks, gate, deadline)
    }

    fn pointer_refusal(&self, what: &'static str) -> AdapterError {
        crate::input::wayland::keysym_send::pointer_refusal(what)
    }

    fn pause(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

pub(crate) fn deliver(
    bus: &SharedBus,
    action: &Action,
    element: Option<&AtspiElement>,
    request: &ActionRequest,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    deliver_in(
        &LiveWorld { bus, lease },
        action,
        element,
        request,
        lease.deadline(),
    )
}

pub(crate) fn deliver_in(
    world: &impl PhysicalWorld,
    action: &Action,
    element: Option<&AtspiElement>,
    request: &ActionRequest,
    deadline: Deadline,
) -> Result<ActionResult, AdapterError> {
    ensure_budget(deadline)?;
    let verified = request.verified_point().cloned();
    let steps = match (action, element) {
        (Action::PressKey(combo), None) => {
            physical_keyboard::press_key_global_steps(world, combo, deadline)?
        }
        (Action::KeyDown(_) | Action::KeyUp(_) | Action::Hover | Action::Drag(_), _) => {
            return Err(adapter_level_rejection(action.name()));
        }
        (_, None) => return Err(missing_element(action.name())),
        (Action::TypeText(text), Some(target)) => {
            physical_keyboard::type_text_steps(world, target, text, request.policy, deadline)?
        }
        (Action::PressKey(combo), Some(target)) => physical_keyboard::press_key_element_steps(
            world,
            target,
            combo,
            request.policy,
            deadline,
        )?,
        (Action::Click, Some(target)) => physical_click::click_steps(
            world,
            target,
            ClickSpec::left(1, verified),
            request.policy,
            deadline,
        )?,
        (Action::DoubleClick, Some(target)) => physical_click::click_steps(
            world,
            target,
            ClickSpec::left(2, verified),
            request.policy,
            deadline,
        )?,
        (Action::TripleClick, Some(target)) => physical_click::click_steps(
            world,
            target,
            ClickSpec::left(3, verified),
            request.policy,
            deadline,
        )?,
        (Action::RightClick, Some(target)) => physical_click::click_steps(
            world,
            target,
            ClickSpec::right(verified),
            request.policy,
            deadline,
        )?,
        (Action::Scroll(direction, amount), Some(target)) => physical_scroll::scroll_steps(
            world,
            target,
            physical_scroll::wheel_deltas(direction, *amount),
            request.policy,
            deadline,
        )?,
        (other, Some(_)) => return Err(unsupported_action(other.name())),
    };
    Ok(ActionResult::from_execution(action, steps))
}

pub(crate) fn ensure_budget(deadline: Deadline) -> Result<(), AdapterError> {
    if deadline.is_expired() {
        return Err(AdapterError::timeout("Operation exceeded its deadline")
            .with_details(json!({
                "kind": "deadline",
                "timeout_ms": deadline.timeout_ms(),
                "elapsed_ms": deadline.elapsed().as_millis(),
                "physical_delivery_started": false,
            }))
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(())
}

pub(crate) fn ensure_keyboard_policy(policy: InteractionPolicy) -> Result<(), AdapterError> {
    if !policy.allow_focus_steal {
        return Err(AdapterError::policy_denied_for_policy(
            "Physical keyboard fallback requires focus permission",
            policy,
        ));
    }
    Ok(())
}

pub(crate) fn ensure_headed_click_policy(policy: InteractionPolicy) -> Result<(), AdapterError> {
    if !policy.allow_cursor_move || !policy.allow_focus_steal {
        return Err(AdapterError::policy_denied_for_policy(
            "Physical pointer input is disabled by the current interaction policy",
            policy,
        ));
    }
    Ok(())
}

pub(crate) fn aim_point(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    verified: Option<&Point>,
    deadline: Deadline,
) -> Result<Point, AdapterError> {
    if let Some(point) = verified {
        point.validate()?;
        return Ok(point.clone());
    }
    match world.bounds(element, deadline)? {
        Some(rect) if rect.width > 0.0 && rect.height > 0.0 => {
            let point = Point {
                x: rect.x + rect.width / 2.0,
                y: rect.y + rect.height / 2.0,
            };
            point.validate()?;
            Ok(point)
        }
        _ => Err(geometry_refusal()),
    }
}

fn geometry_refusal() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The pointer position of the target cannot be established, so no event was sent",
    )
    .with_suggestion("Run 'snapshot' to refresh, or use the semantic action instead")
    .with_details(json!({
        "check": "geometry",
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn adapter_level_rejection(label: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("{label} requires adapter-level handling, not element action"),
    )
    .with_suggestion(
        "Use the top-level command (e.g. 'hover', 'drag', 'press') instead of targeting an element.",
    )
    .with_details(json!({ "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn unsupported_action(label: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("{label} has no physical delivery on Linux"),
    )
    .with_details(json!({ "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn missing_element(label: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        format!("{label} needs a target element for physical delivery"),
    )
    .with_details(json!({ "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "physical_target_tests.rs"]
pub(crate) mod tests;
