//! Reads only a compositor's registry, for a caller that needs to know which
//! globals a socket advertises before it binds any of them.

use std::os::unix::net::UnixStream;

use agent_desktop_core::Deadline;
use wayland_client::delegate_dispatch;
use wayland_client::protocol::{wl_callback, wl_registry};

use super::exchange::{Exchange, ExchangeHandler, HoldsExchange};
use super::link::Link;
use super::wayland_fault::WaylandFault;

#[derive(Default)]
struct Registry {
    exchange: Exchange,
}

impl HoldsExchange for Registry {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

delegate_dispatch!(Registry: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(Registry: [wl_callback::WlCallback: u32] => ExchangeHandler);

/// The globals the compositor behind `stream` advertises.
pub(crate) fn read_registry(
    stream: UnixStream,
    deadline: Deadline,
) -> Result<Exchange, WaylandFault> {
    let mut registry = Registry::default();
    Link::over(stream, &mut registry, deadline)?;
    Ok(registry.exchange)
}

#[cfg(test)]
#[path = "registry_read_tests.rs"]
mod tests;
