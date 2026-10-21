use agent_desktop_core::{
    ActionStep, AdapterError, Deadline, InteractionPolicy, MouseButton, Point, StepMechanism,
    validate_mouse_click_count,
};

use super::physical_target::{PhysicalWorld, aim_point, ensure_budget, ensure_headed_click_policy};
use crate::input::backend::Backend;
use crate::tree::AtspiElement;

const CLICK_LABEL: &str = "XTEST.click";

pub(crate) struct ClickSpec {
    pub(crate) button: MouseButton,
    pub(crate) count: u32,
    pub(crate) verified: Option<Point>,
}

impl ClickSpec {
    pub(crate) fn left(count: u32, verified: Option<Point>) -> Self {
        Self {
            button: MouseButton::Left,
            count,
            verified,
        }
    }

    pub(crate) fn right(verified: Option<Point>) -> Self {
        Self {
            button: MouseButton::Right,
            count: 1,
            verified,
        }
    }
}

pub(crate) fn click_steps(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    spec: ClickSpec,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<ActionStep>, AdapterError> {
    ensure_headed_click_policy(policy)?;
    ensure_budget(deadline)?;
    if world.backend()? == Backend::Wayland {
        return Err(world.pointer_refusal("click"));
    }
    let at = aim_point(world, element, spec.verified.as_ref(), deadline)?;
    let map = world.keymap(deadline)?;
    validate_mouse_click_count(spec.count)?;
    let plans = (0..spec.count)
        .map(|_| world.plan_click(&map, at.clone(), &spec.button, 1))
        .collect::<Result<Vec<_>, _>>()?;
    let mut next = 0u32;
    world.send_plans(
        plans,
        &mut || {
            world
                .validate_pointer(element, &at, deadline)
                .map_err(|mut error| {
                    let details = error.details.get_or_insert_with(|| serde_json::json!({}));
                    details["delivered_clicks"] = next.into();
                    error
                })?;
            next += 1;
            Ok(())
        },
        deadline,
    )?;
    Ok(vec![
        ActionStep::succeeded(CLICK_LABEL)
            .with_mechanism(StepMechanism::PhysicalSynthetic)
            .with_verified(false),
    ])
}

#[cfg(test)]
#[path = "physical_click_tests.rs"]
mod tests;
