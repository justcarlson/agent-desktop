//! The display layout of a Wayland compositor that is not Mutter, read over
//! `wl_output` and `zxdg_output_manager_v1`. wlroots-family compositors such
//! as Sway and Hyprland advertise both; a compositor without them is refused
//! by the capability probe before any output is bound.

use std::path::Path;

use agent_desktop_core::Deadline;
use wayland_client::protocol::{wl_callback, wl_output, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, delegate_dispatch, delegate_noop};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

use super::output_probe::OutputProbe;
use super::wayland_output::WaylandOutput;
use crate::system::wayland::hyprland::exchange::{Exchange, ExchangeHandler, HoldsExchange};
use crate::system::wayland::hyprland::global::Global;
use crate::system::wayland::hyprland::link::Link;

const OUTPUT: &str = "wl_output";
const MANAGER: &str = "zxdg_output_manager_v1";
const OUTPUT_VERSION: u32 = 4;
const MANAGER_VERSION: u32 = 3;

#[derive(Default)]
struct OutputState {
    exchange: Exchange,
    outputs: Vec<WaylandOutput>,
}

impl HoldsExchange for OutputState {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

delegate_dispatch!(OutputState: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(OutputState: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_noop!(OutputState: ignore ZxdgOutputManagerV1);

fn turns(transform: WEnum<wl_output::Transform>) -> bool {
    use wl_output::Transform;
    matches!(
        transform,
        WEnum::Value(
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270
        )
    )
}

fn is_current(flags: WEnum<wl_output::Mode>) -> bool {
    matches!(flags, WEnum::Value(flags) if flags.contains(wl_output::Mode::Current))
}

impl Dispatch<wl_output::WlOutput, usize> for OutputState {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(output) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            wl_output::Event::Geometry { transform, .. } => output.turned = turns(transform),
            wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } if is_current(flags) => output.mode = Some((width, height)),
            wl_output::Event::Scale { factor } => output.integer_scale = factor,
            wl_output::Event::Name { name } if output.name.is_none() => output.name = Some(name),
            _ => {}
        }
    }
}

impl Dispatch<ZxdgOutputV1, usize> for OutputState {
    fn event(
        state: &mut Self,
        _: &ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(output) = state.outputs.get_mut(*index) else {
            return;
        };
        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => output.position = Some((x, y)),
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                output.logical = Some((width, height))
            }
            zxdg_output_v1::Event::Name { name } => output.name = Some(name),
            _ => {}
        }
    }
}

/// The capability probe: the globals the read binds, or the names of the
/// ones the compositor does not advertise.
fn advertised(exchange: &Exchange) -> Result<(Global, Vec<Global>), OutputProbe> {
    let manager = exchange.find(MANAGER, 1).cloned();
    let outputs: Vec<Global> = exchange.all(OUTPUT).cloned().collect();
    match manager {
        Some(manager) if !outputs.is_empty() => Ok((manager, outputs)),
        manager => {
            let mut missing = Vec::new();
            if outputs.is_empty() {
                missing.push(OUTPUT);
            }
            if manager.is_none() {
                missing.push(MANAGER);
            }
            Err(OutputProbe::Unadvertised(missing))
        }
    }
}

/// Every output of the compositor behind `socket`, in advertisement order,
/// read in two round trips: the registry, then the binds and their events.
pub(crate) fn read_outputs(
    socket: &Path,
    deadline: Deadline,
) -> Result<Vec<WaylandOutput>, OutputProbe> {
    let mut state = OutputState::default();
    let mut link = Link::open(socket, &mut state, deadline).map_err(OutputProbe::Wayland)?;
    let (manager_global, output_globals) = advertised(&state.exchange)?;
    let manager: ZxdgOutputManagerV1 = link.bind(&manager_global, MANAGER_VERSION, ());
    let handle = link.handle();
    let mut bound = Vec::with_capacity(output_globals.len());
    for (index, global) in output_globals.iter().enumerate() {
        state.outputs.push(WaylandOutput::default());
        let output: wl_output::WlOutput = link.bind(global, OUTPUT_VERSION, index);
        let xdg = manager.get_xdg_output(&output, &handle, index);
        bound.push((output, xdg));
    }
    link.roundtrip(&mut state).map_err(OutputProbe::Wayland)?;
    drop(bound);
    Ok(state.outputs)
}

#[cfg(test)]
#[path = "display_outputs_tests.rs"]
mod tests;
