//! The origin of a bound toplevel's window coordinates in the global
//! layout, which is where an AT-SPI window rectangle is placed for bounds.

use crate::system::hyprland::clients::hypr_client::HyprClient;

/// `client`'s compositor origin in whole logical pixels, or `None` when its
/// geometry is not a finite position.
pub(crate) fn origin_of(client: &HyprClient) -> Option<(i32, i32)> {
    Some((pixel(client.geometry.x)?, pixel(client.geometry.y)?))
}

fn pixel(value: f64) -> Option<i32> {
    let rounded = value.round();
    (rounded.is_finite() && rounded >= f64::from(i32::MIN) && rounded <= f64::from(i32::MAX))
        .then_some(rounded as i32)
}

#[cfg(test)]
#[path = "frame_origin_tests.rs"]
mod tests;
