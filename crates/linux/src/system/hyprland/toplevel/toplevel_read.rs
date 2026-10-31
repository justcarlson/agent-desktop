//! A one-shot read of the bound compositor's toplevels. `ToplevelReader`
//! holds the connection for a caller that needs the handles afterwards.

use agent_desktop_core::{AdapterError, Deadline};

use super::toplevel_reader::ToplevelReader;
use super::toplevel_view::ToplevelView;
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;

/// Every toplevel of the bound compositor that has an identifier and a
/// Hyprland address, in the compositor's announce order.
pub(crate) fn read_toplevels(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<Vec<ToplevelView>, AdapterError> {
    ToplevelReader::open(binding, deadline)?.views()
}

#[cfg(test)]
#[path = "toplevel_read_tests.rs"]
mod tests;
