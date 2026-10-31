use agent_desktop_core::Deadline;
use wayland_client::delegate_dispatch;
use wayland_client::protocol::wl_callback;
use wayland_client::protocol::wl_registry;

use super::*;
use crate::system::wayland::exchange::{ExchangeHandler, HoldsExchange};
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::fake_toplevels::FakeToplevel;
use crate::system::wayland::link::Link;

#[derive(Default)]
struct Seen {
    exchange: Exchange,
    toplevels: Vec<WlrToplevel>,
}

impl HoldsExchange for Seen {
    fn exchange(&self) -> &Exchange {
        &self.exchange
    }

    fn exchange_mut(&mut self) -> &mut Exchange {
        &mut self.exchange
    }
}

impl HoldsWlrToplevels for Seen {
    fn wlr_toplevels_mut(&mut self) -> &mut Vec<WlrToplevel> {
        &mut self.toplevels
    }
}

delegate_dispatch!(Seen: [wl_registry::WlRegistry: ()] => ExchangeHandler);
delegate_dispatch!(Seen: [wl_callback::WlCallback: u32] => ExchangeHandler);
delegate_dispatch!(Seen: [ZwlrForeignToplevelManagerV1: ()] => WlrToplevelHandler);
delegate_dispatch!(Seen: [ZwlrForeignToplevelHandleV1: ()] => WlrToplevelHandler);
delegate_dispatch!(Seen: [WlSeat: ()] => WlrToplevelHandler);

fn open(toplevels: Vec<FakeToplevel>) -> (FakeCompositor, Link<Seen>, Seen) {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: vec![(SEAT_INTERFACE, 7), (WLR_MANAGER_INTERFACE, 3)],
        toplevels,
    });
    let mut seen = Seen::default();
    let deadline = Deadline::after(2_000).expect("deadline");
    let mut link = Link::open(&compositor.socket, &mut seen, deadline).expect("opens");
    let manager = advertised_manager(&seen.exchange)
        .cloned()
        .expect("advertised");
    let _manager: ZwlrForeignToplevelManagerV1 = link.bind(&manager, WLR_MANAGER_VERSION, ());
    link.roundtrip(&mut seen).expect("answers");
    (compositor, link, seen)
}

fn toplevel(identifier: &'static str) -> FakeToplevel {
    FakeToplevel {
        identifier,
        address: Some(0x10),
        ..FakeToplevel::default()
    }
}

#[test]
fn the_activated_state_is_read_from_native_endian_words() {
    let words = |values: &[u32]| -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect()
    };
    assert!(holds_activated(&words(&[0, 2])));
    assert!(!holds_activated(&words(&[0, 1, 3])));
    assert!(!holds_activated(&[]));
}

#[test]
fn every_handle_reports_its_activated_state() {
    let (_compositor, _link, seen) = open(vec![
        FakeToplevel {
            activated: true,
            ..toplevel("main")
        },
        toplevel("save"),
    ]);
    assert!(seen.toplevels[0].activated);
    assert!(!seen.toplevels[1].activated);
}

#[test]
fn an_activation_reaches_the_compositor_and_its_answer_is_read() {
    let (compositor, mut link, mut seen) = open(vec![
        FakeToplevel {
            activated: true,
            ..toplevel("one")
        },
        toplevel("two"),
    ]);
    let seat_global = seen
        .exchange
        .find(SEAT_INTERFACE, 1)
        .cloned()
        .expect("seat");
    let seat: WlSeat = link.bind(&seat_global, 1, ());
    seen.toplevels[1].handle.activate(&seat);
    link.roundtrip(&mut seen).expect("answers");
    assert!(!seen.toplevels[0].activated);
    assert!(seen.toplevels[1].activated);
    assert_eq!(
        *compositor.log.lock().expect("log"),
        vec!["activate two".to_owned()]
    );
}

#[test]
fn the_manager_is_found_from_its_registry_entry_alone() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![
            ("wl_compositor", 6),
            ("hyprland_toplevel_mapping_manager_v1", 1),
        ],
        outputs: vec![],
    });
    let mut seen = Seen::default();
    let deadline = Deadline::after(2_000).expect("deadline");
    let _link = Link::open(&compositor.socket, &mut seen, deadline).expect("opens");
    assert!(advertised_manager(&seen.exchange).is_none());
}
