use std::time::Instant;

use agent_desktop_core::Deadline;
use wayland_client::delegate_dispatch;
use wayland_client::protocol::{wl_callback, wl_registry};

use super::*;
use crate::system::wayland::hyprland::exchange::{Exchange, ExchangeHandler};
use crate::system::wayland::hyprland::fake_compositor::{FakeCompositor, Script};

#[derive(Default)]
struct Probe {
    exchange: Exchange,
}

impl HoldsExchange for Probe {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

delegate_dispatch!(Probe: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(Probe: [wl_callback::WlCallback: u32] => ExchangeHandler);

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

#[test]
fn opening_a_link_reads_every_advertised_global_with_its_version() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![("wl_compositor", 6), ("zxdg_output_manager_v1", 3)],
        outputs: vec![],
    });
    let mut probe = Probe::default();
    let mut link = Link::open(&compositor.socket, &mut probe, deadline(2_000)).expect("opens");
    let manager = probe
        .exchange
        .find("zxdg_output_manager_v1", 1)
        .expect("advertised");
    assert_eq!((manager.name, manager.version), (2, 3));
    assert!(probe.exchange.find("zxdg_output_manager_v1", 4).is_none());
    assert!(probe.exchange.find("wl_output", 1).is_none());
    link.roundtrip(&mut probe)
        .expect("a second round trip answers");
}

#[test]
fn a_compositor_that_accepts_and_never_answers_is_silent_within_the_deadline() {
    let compositor = FakeCompositor::start(Script::Silent);
    let started = Instant::now();
    let fault = Link::open(&compositor.socket, &mut Probe::default(), deadline(200)).err();
    assert_eq!(fault, Some(WaylandFault::Silent));
    assert!(
        started.elapsed().as_millis() < 1_200,
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_compositor_that_hangs_up_is_gone() {
    let compositor = FakeCompositor::start(Script::HangUp);
    let fault = Link::open(&compositor.socket, &mut Probe::default(), deadline(2_000)).err();
    assert_eq!(fault, Some(WaylandFault::Gone));
}
