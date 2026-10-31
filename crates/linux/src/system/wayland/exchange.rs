//! The registry and round-trip bookkeeping every reader's state carries. A
//! reader's state holds an `Exchange`, exposes it through `HoldsExchange`, and
//! delegates the registry and callback events here with
//! `wayland_client::delegate_dispatch!`.

use wayland_client::protocol::{wl_callback, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::global::Global;

/// The globals advertised so far and the last round trip that finished.
#[derive(Debug, Default)]
pub(crate) struct Exchange {
    globals: Vec<Global>,
    finished: u32,
}

impl Exchange {
    /// Every advertised global of `interface`, in advertisement order.
    pub(crate) fn all<'a>(&'a self, interface: &'a str) -> impl Iterator<Item = &'a Global> + 'a {
        self.globals
            .iter()
            .filter(move |global| global.interface == interface)
    }

    /// The first advertised global of `interface` that serves at least
    /// `version`.
    pub(crate) fn find(&self, interface: &str, version: u32) -> Option<&Global> {
        self.globals
            .iter()
            .find(|global| global.interface == interface && global.version >= version)
    }

    pub(crate) fn has_finished(&self, round: u32) -> bool {
        self.finished >= round
    }

    #[cfg(test)]
    pub(crate) fn advertising(globals: Vec<Global>) -> Self {
        Self {
            globals,
            finished: 0,
        }
    }
}

/// A reader state that carries an `Exchange`.
pub(crate) trait HoldsExchange {
    fn exchange(&self) -> &Exchange;
    fn exchange_mut(&mut self) -> &mut Exchange;
}

/// Handles the registry and the round-trip callbacks for any reader state.
pub(crate) struct ExchangeHandler;

impl<S> Dispatch<wl_registry::WlRegistry, (), S> for ExchangeHandler
where
    S: Dispatch<wl_registry::WlRegistry, ()> + HoldsExchange,
{
    fn event(
        state: &mut S,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        let exchange = state.exchange_mut();
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => exchange.globals.push(Global {
                name,
                interface,
                version,
            }),
            wl_registry::Event::GlobalRemove { name } => {
                exchange.globals.retain(|global| global.name != name)
            }
            _ => {}
        }
    }
}

impl<S> Dispatch<wl_callback::WlCallback, u32, S> for ExchangeHandler
where
    S: Dispatch<wl_callback::WlCallback, u32> + HoldsExchange,
{
    fn event(
        state: &mut S,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        round: &u32,
        _: &Connection,
        _: &QueueHandle<S>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            let exchange = state.exchange_mut();
            exchange.finished = exchange.finished.max(*round);
        }
    }
}
