//! Click and scroll by ref on the bound Hyprland instance. The aim point is
//! the element's window rectangle placed at its toplevel's compositor
//! origin. Before any event the receiver gate must pass, and today it
//! refuses every target: the compositor's stack cannot see the surfaces
//! inside a client (probes/linux/FINDINGS.md L47-4, L47-5, L45-2). The
//! stack proof before every burst stays as the check a passing gate would
//! still need.

use agent_desktop_core::{ActionStep, AdapterError, Deadline, StepMechanism};

use crate::input::keyboard::keymap::Keymap;
use crate::input::mouse::gesture::Gesture;
use crate::input::mouse::wheel_plan::plan_wheel;
use crate::input::physical::target_pointer::{ClickSpec, send_bursts_with};
use crate::input::physical::target_world::PhysicalWorld;
use crate::system::SharedBus;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::placement::placed_target::PlacedTarget;
use crate::system::hyprland::placement::placement_read::place_target;
use crate::system::hyprland::receiver::receiver_read::prove_receiver;

use super::native_receiver::refuse_unproven_receiver;
use crate::tree::AtspiElement;

/// One bound instance and the element a ref pointer action aims at.
pub(crate) struct RefPointer<'a> {
    pub(crate) binding: &'a HyprlandBinding,
    pub(crate) element: &'a AtspiElement,
    pub(crate) deadline: Deadline,
}

impl RefPointer<'_> {
    pub(crate) fn click(
        &self,
        world: &impl PhysicalWorld,
        spec: &ClickSpec,
    ) -> Result<Vec<ActionStep>, AdapterError> {
        agent_desktop_core::validate_mouse_click_count(spec.count)?;
        let bus = SharedBus::default();
        let target = place_target(
            self.binding,
            &bus,
            self.element,
            spec.verified.as_ref(),
            self.deadline,
        )?;
        self.click_placed(world, spec, &target)
    }

    /// Clicks at a placed target once its receiver is proven.
    pub(crate) fn click_placed(
        &self,
        world: &impl PhysicalWorld,
        spec: &ClickSpec,
        target: &PlacedTarget,
    ) -> Result<Vec<ActionStep>, AdapterError> {
        refuse_unproven_receiver(self.binding, target.address, self.deadline)?;
        let plan = world.plan_click(
            &Keymap::default(),
            target.at.clone(),
            &spec.button,
            spec.count,
        )?;
        send_bursts_with(
            world,
            Gesture::clicks(vec![plan]),
            self.deadline,
            &mut |check| prove_receiver(self.binding, target.address, &target.at, check),
        )?;
        Ok(vec![step("hyprland_virtual_pointer.click")])
    }

    pub(crate) fn scroll(
        &self,
        world: &impl PhysicalWorld,
        deltas: (f64, f64),
    ) -> Result<Vec<ActionStep>, AdapterError> {
        let bus = SharedBus::default();
        let target = place_target(self.binding, &bus, self.element, None, self.deadline)?;
        self.scroll_placed(world, deltas, &target)
    }

    /// Scrolls at a placed target once its receiver is proven.
    pub(crate) fn scroll_placed(
        &self,
        world: &impl PhysicalWorld,
        deltas: (f64, f64),
        target: &PlacedTarget,
    ) -> Result<Vec<ActionStep>, AdapterError> {
        refuse_unproven_receiver(self.binding, target.address, self.deadline)?;
        let plans = plan_wheel(
            &Keymap::default(),
            target.at.clone(),
            deltas.0,
            deltas.1,
            &[],
        )?;
        send_bursts_with(
            world,
            Gesture::unbounded(plans),
            self.deadline,
            &mut |check| prove_receiver(self.binding, target.address, &target.at, check),
        )?;
        Ok(vec![step("hyprland_virtual_pointer.scroll")])
    }
}

fn step(label: &'static str) -> ActionStep {
    ActionStep::succeeded(label)
        .with_mechanism(StepMechanism::PhysicalSynthetic)
        .with_verified(false)
}

#[cfg(test)]
#[path = "ref_pointer_tests.rs"]
pub(crate) mod tests;
