//! One read of the bound compositor's clients and toplevels that a closed
//! window's reused address cannot cross.

use agent_desktop_core::{AdapterError, Deadline};

use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::clients::client_read::read_clients;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_read::read_toplevels;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

/// The clients and toplevels of the bound compositor. Clients and toplevels
/// are joined only by address, and Hyprland can hand a closed window's
/// address to a new one between two reads, so the clients are read on both
/// sides of the toplevel read and only those both reads agree on are kept.
pub(crate) fn read_inventory(
    binding: &HyprlandBinding,
    deadline: Deadline,
) -> Result<(Vec<HyprClient>, Vec<ToplevelView>), AdapterError> {
    stable_inventory(
        || read_clients(binding, deadline),
        || read_toplevels(binding, deadline),
    )
}

/// Reads the clients, then the toplevels, then the clients again, and keeps
/// the later clients whose address names the same process and class in the
/// earlier read. Titles are not compared, because a terminal or browser
/// retitles its window between two reads; a reused address names a new
/// window, whose process or class differs.
pub(crate) fn stable_inventory<T>(
    mut clients: impl FnMut() -> Result<Vec<HyprClient>, AdapterError>,
    toplevels: impl FnOnce() -> Result<Vec<T>, AdapterError>,
) -> Result<(Vec<HyprClient>, Vec<T>), AdapterError> {
    let before = clients()?;
    let toplevels = toplevels()?;
    let after = clients()?;
    Ok((stable_clients(&before, after), toplevels))
}

fn stable_clients(before: &[HyprClient], after: Vec<HyprClient>) -> Vec<HyprClient> {
    after
        .into_iter()
        .filter(|client| {
            before.iter().any(|earlier| {
                earlier.address == client.address
                    && earlier.pid == client.pid
                    && earlier.class == client.class
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "inventory_read_tests.rs"]
mod tests;
