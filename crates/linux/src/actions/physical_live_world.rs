use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, HitTestResult, InteractionLease,
    KeyCombo, MouseButton, Point, Rect,
};
use serde_json::json;

use super::physical_target::PhysicalWorld;
use crate::input::backend::Backend;
use crate::input::delivery::{Delivery, Gate};
use crate::input::keymap::Keymap;
use crate::input::keysym_step::KeysymStep;
use crate::input::plan::Plan;
use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) struct LiveWorld<'a> {
    pub(crate) bus: &'a SharedBus,
    pub(crate) lease: &'a InteractionLease,
}

impl PhysicalWorld for LiveWorld<'_> {
    fn backend(&self) -> Result<Backend, AdapterError> {
        crate::input::backend::backend()
    }

    fn focus_target(&self, element: &AtspiElement, deadline: Deadline) -> Result<(), AdapterError> {
        super::keyboard_focus::grab(self.bus, element, deadline)
    }

    fn target_focused(
        &self,
        element: &AtspiElement,
        deadline: Deadline,
    ) -> Result<bool, AdapterError> {
        super::keyboard_focus::read(self.bus, element, deadline)
    }

    fn keymap(&self, _deadline: Deadline) -> Result<Keymap, AdapterError> {
        match self.backend()? {
            Backend::X11(display) => crate::input::x11::session::live_keymap(&display, self.lease),
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

    fn validate_pointer(
        &self,
        element: &AtspiElement,
        point: &Point,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let placed = crate::tree::live_offscreen::placement(self.bus, element, deadline);
        super::physical_aim::validate(point, &placed, element.scroll_container().is_some())?;
        let handle = element.clone().into_native_handle();
        match crate::tree::hit_test(self.bus, &handle, point.clone(), deadline)? {
            HitTestResult::ReachesTarget => Ok(()),
            HitTestResult::InterceptedBy { role, name, bounds } => Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "Another element intercepts the pointer target",
            )
            .with_details(json!({
                "check": "receives_events",
                "interceptor": { "role": role, "name": name, "bounds": bounds },
                "physical_delivery_started": false,
            }))
            .with_suggestion("Reveal the target and refresh the snapshot before retrying")
            .with_disposition(DeliverySemantics::not_delivered())),
            hit => Err(
                AdapterError::new(ErrorCode::ActionFailed, "Pointer delivery is unproven")
                    .with_details(json!({
                        "kind": "pointer_hit_unproven",
                        "hit_test": hit,
                        "physical_delivery_started": false,
                    }))
                    .with_suggestion("Expose readable, unobstructed target geometry and retry")
                    .with_disposition(DeliverySemantics::not_delivered()),
            ),
        }
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
