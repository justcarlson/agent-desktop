use agent_desktop_core::{
    ActionStep, AdapterError, Deadline, InteractionPolicy, MouseButton, Point, StepMechanism,
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
    let plan = world.plan_click(&map, at.clone(), &spec.button, spec.count)?;
    world.send_plans(
        vec![plan],
        &mut || world.validate_pointer(element, &at, deadline),
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
