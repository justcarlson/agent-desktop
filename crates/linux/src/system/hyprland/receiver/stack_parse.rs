//! Parsers for the `j/clients` and `j/layers` replies as receiver evidence.
//! An entry the parser cannot read makes the whole reply unusable, because
//! the unread entry could be the cover.

use super::layer_info::LayerInfo;
use super::stack_client::StackClient;

/// Every client of a `j/clients` reply with its stacking state.
pub(crate) fn parse_stack_clients(body: &[u8]) -> Option<Vec<StackClient>> {
    let _ = body;
    None
}

/// Every layer surface of a `j/layers` reply, on every output and level.
pub(crate) fn parse_layers(body: &[u8]) -> Option<Vec<LayerInfo>> {
    let _ = body;
    None
}

#[cfg(test)]
#[path = "stack_parse_tests.rs"]
mod tests;
