//! The outputs the renderer draws on and the events that describe them.
//! Every change to an output's origin, size, mode or scale, and every
//! pinned cursor surface the compositor closes, as it does when its output
//! goes, marks the layout changed, so the renderer places every cursor again.

use wayland_client::backend::ObjectId;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1;

use super::layer_state::LayerState;
use super::output_layout::OutputLayout;
use super::tracked_output::TrackedOutput;

/// The bound outputs, the pinned surfaces closed since the last look, and
/// whether the layout changed since the last look.
#[derive(Debug, Default)]
pub(crate) struct LayerOutputs {
    pub(crate) tracked: Vec<TrackedOutput>,
    pub(crate) closed_layers: Vec<ObjectId>,
    pub(crate) changed: bool,
}

impl LayerOutputs {
    /// Every output whose place in the layout is known.
    pub(crate) fn layout(&self, fractional_manager: bool) -> Vec<OutputLayout> {
        self.tracked
            .iter()
            .filter_map(|output| output.layout(fractional_manager))
            .collect()
    }

    /// The `wl_output` with registry name `name`.
    pub(crate) fn proxy(&self, name: u32) -> Option<&wl_output::WlOutput> {
        self.tracked
            .iter()
            .find(|output| output.name == name)
            .map(|output| &output.output)
    }

    fn update(
        &mut self,
        found: impl Fn(&TrackedOutput) -> bool,
        change: impl FnOnce(&mut TrackedOutput),
    ) {
        let Some(output) = self.tracked.iter_mut().find(|output| found(output)) else {
            return;
        };
        let before = (output.geometry.clone(), output.scale.clone());
        change(output);
        if before != (output.geometry.clone(), output.scale.clone()) {
            self.changed = true;
        }
    }

    fn named(&mut self, name: u32, change: impl FnOnce(&mut TrackedOutput)) {
        self.update(|output| output.name == name, change);
    }
}

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

impl Dispatch<wl_output::WlOutput, ()> for LayerState {
    fn event(
        state: &mut Self,
        proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let found = |output: &TrackedOutput| output.output == *proxy;
        match event {
            wl_output::Event::Geometry { transform, .. } => state
                .outputs
                .update(found, |output| output.geometry.turned = turns(transform)),
            wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } if is_current(flags) => state
                .outputs
                .update(found, |output| output.geometry.mode = Some((width, height))),
            wl_output::Event::Scale { factor } => state
                .outputs
                .update(found, |output| output.scale.integer = Some(factor)),
            _ => {}
        }
    }
}

impl Dispatch<zxdg_output_v1::ZxdgOutputV1, u32> for LayerState {
    fn event(
        state: &mut Self,
        _: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => state
                .outputs
                .named(*name, |output| output.geometry.origin = Some((x, y))),
            zxdg_output_v1::Event::LogicalSize { width, height } => state
                .outputs
                .named(*name, |output| output.geometry.size = Some((width, height))),
            _ => {}
        }
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, u32> for LayerState {
    fn event(
        state: &mut Self,
        _: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            state
                .outputs
                .named(*name, |output| output.scale.fractional = Some(scale));
        }
    }
}

impl Dispatch<wl_surface::WlSurface, Option<u32>> for LayerState {
    fn event(
        state: &mut Self,
        _: &wl_surface::WlSurface,
        event: wl_surface::Event,
        pinned: &Option<u32>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let (wl_surface::Event::PreferredBufferScale { factor }, Some(name)) = (event, pinned) {
            state
                .outputs
                .named(*name, |output| output.scale.buffer = Some(factor));
        }
    }
}
