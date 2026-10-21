use agent_desktop_core::{
    ActionStep, AdapterError, Deadline, Direction, InteractionPolicy, StepMechanism,
};

use super::physical_target::{PhysicalWorld, aim_point, ensure_budget, ensure_headed_click_policy};
use crate::input::backend::Backend;
use crate::tree::AtspiElement;

const SCROLL_LABEL: &str = "XTEST.scroll";
const NOTCHES_PER_AMOUNT: f64 = 1.0;

/// Wheel deltas for a scroll request. A positive vertical delta is a wheel
/// turn away from the user, which moves the content down; a positive
/// horizontal delta moves the content to the right.
pub(crate) fn wheel_deltas(direction: &Direction, amount: u32) -> (f64, f64) {
    let notches = f64::from(amount) * NOTCHES_PER_AMOUNT;
    match direction {
        Direction::Up => (0.0, notches),
        Direction::Down => (0.0, -notches),
        Direction::Left => (-notches, 0.0),
        Direction::Right => (notches, 0.0),
    }
}

pub(crate) fn scroll_steps(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    deltas: (f64, f64),
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<ActionStep>, AdapterError> {
    ensure_headed_click_policy(policy)?;
    ensure_budget(deadline)?;
    if world.backend()? == Backend::Wayland {
        return Err(world.pointer_refusal("scroll"));
    }
    let at = aim_point(world, element, None, deadline)?;
    let map = world.keymap(deadline)?;
    let plans = world.plan_wheel(&map, at.clone(), deltas)?;
    world.send_plans(
        plans,
        &mut || world.validate_pointer(element, &at, deadline),
        deadline,
    )?;
    Ok(vec![
        ActionStep::succeeded(SCROLL_LABEL)
            .with_mechanism(StepMechanism::PhysicalSynthetic)
            .with_verified(false),
    ])
}

#[cfg(test)]
#[path = "physical_scroll_tests.rs"]
mod tests;
