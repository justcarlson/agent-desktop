#![expect(
    dead_code,
    reason = "the module is laid out ahead of the code that calls it"
)]

use agent_desktop_core::{
    Action, ActionResult, ActionStep, AdapterError, Deadline, InteractionPolicy, KeyCombo,
    StepMechanism,
};

use super::frame_active::ActiveGate;
use super::physical_target::{LiveWorld, PhysicalWorld, ensure_budget, ensure_keyboard_policy};
use crate::input::backend::Backend;
use crate::system::SharedBus;
use crate::tree::AtspiElement;

pub(crate) fn type_text_steps(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    text: &str,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<ActionStep>, AdapterError> {
    ensure_keyboard_policy(policy)?;
    ensure_budget(deadline)?;
    let backend = world.backend()?;
    let mut gate = ActiveGate::new(world, element, deadline, backend == Backend::Wayland);
    match &backend {
        Backend::X11(_) => {
            let map = world.keymap(deadline)?;
            let plans = world.plan_text(&map, text)?;
            gate.check()?;
            world.send_plans(plans, &mut || gate.check(), deadline)?;
        }
        Backend::Wayland => {
            for chunk in world.plan_text_keysyms(text) {
                world.send_keysyms(&chunk, &mut || gate.check(), deadline)?;
            }
        }
    }
    Ok(vec![physical_step(&backend, "type_text")])
}

pub(crate) fn press_key_element_steps(
    world: &impl PhysicalWorld,
    element: &AtspiElement,
    combo: &KeyCombo,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<ActionStep>, AdapterError> {
    ensure_keyboard_policy(policy)?;
    ensure_budget(deadline)?;
    let backend = world.backend()?;
    let mut gate = ActiveGate::new(world, element, deadline, backend == Backend::Wayland);
    if matches!(backend, Backend::X11(_)) {
        gate.check()?;
    }
    send_combo(world, &backend, combo, &mut || gate.check(), deadline)?;
    Ok(vec![physical_step(&backend, "press_key")])
}

pub(crate) fn press_key_global_steps(
    world: &impl PhysicalWorld,
    combo: &KeyCombo,
    deadline: Deadline,
) -> Result<Vec<ActionStep>, AdapterError> {
    ensure_budget(deadline)?;
    let backend = world.backend()?;
    send_combo(world, &backend, combo, &mut || Ok(()), deadline)?;
    Ok(vec![physical_step(&backend, "press_key")])
}

pub(crate) fn press_global(
    bus: &SharedBus,
    combo: &KeyCombo,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<ActionResult, AdapterError> {
    ensure_keyboard_policy(policy)?;
    let steps = press_key_global_steps(&LiveWorld { bus }, combo, deadline)?;
    Ok(ActionResult::from_execution(
        &Action::PressKey(combo.clone()),
        steps,
    ))
}

fn send_combo(
    world: &impl PhysicalWorld,
    backend: &Backend,
    combo: &KeyCombo,
    gate: &mut dyn FnMut() -> Result<(), AdapterError>,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    match backend {
        Backend::X11(_) => {
            let map = world.keymap(deadline)?;
            let plan = world.plan_combo(&map, combo)?;
            world.send_plans(vec![plan], gate, deadline)?;
        }
        Backend::Wayland => {
            let keys = world.plan_keysyms(combo)?;
            world.send_keysyms(&keys, gate, deadline)?;
        }
    }
    Ok(())
}

fn physical_step(backend: &Backend, operation: &'static str) -> ActionStep {
    let label = match (backend, operation) {
        (Backend::X11(_), "type_text") => "XTEST.type_text",
        (Backend::X11(_), _) => "XTEST.press_key",
        (Backend::Wayland, "type_text") => "RemoteDesktop.type_text",
        (Backend::Wayland, _) => "RemoteDesktop.press_key",
    };
    ActionStep::succeeded(label)
        .with_mechanism(StepMechanism::PhysicalSynthetic)
        .with_verified(false)
}

#[cfg(test)]
#[path = "physical_keyboard_tests.rs"]
mod tests;
