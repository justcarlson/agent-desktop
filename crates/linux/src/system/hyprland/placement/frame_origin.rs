//! The origin of a bound toplevel's window coordinates in the global
//! layout, which is where an AT-SPI window rectangle is placed for bounds.

use crate::system::hyprland::clients::hypr_client::HyprClient;

/// `client`'s compositor origin in whole logical pixels, or `None` when its
/// geometry is not a finite position.
pub(crate) fn origin_of(_client: &HyprClient) -> Option<(i32, i32)> {
    None
}

#[cfg(test)]
#[path = "frame_origin_tests.rs"]
mod tests;
