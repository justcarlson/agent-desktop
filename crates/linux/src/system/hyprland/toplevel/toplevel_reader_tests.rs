use agent_desktop_core::Deadline;
use wayland_client::Proxy;

use super::*;
use crate::system::hyprland::toplevel::fake_binding::binding_for;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::fake_toplevels::FakeToplevel;

fn reader() -> (FakeCompositor, ToplevelReader) {
    let compositor = FakeCompositor::start(Script::Toplevels {
        globals: vec![(LIST_INTERFACE, 1), (MAPPING_INTERFACE, 1)],
        toplevels: vec![
            FakeToplevel {
                identifier: "a1",
                address: Some(0x10),
                ..FakeToplevel::default()
            },
            FakeToplevel {
                identifier: "b2",
                address: Some(0x20),
                ..FakeToplevel::default()
            },
        ],
    });
    let deadline = Deadline::after(2_000).expect("deadline");
    let reader = ToplevelReader::open(&binding_for(&compositor), deadline).expect("opens");
    (compositor, reader)
}

#[test]
fn the_reader_keeps_the_handle_of_each_toplevel_it_read() {
    let (_compositor, reader) = reader();
    let views = reader.views().expect("views");
    assert_eq!(views.len(), 2);
    for view in &views {
        let handle = reader.ext_handle(&view.identifier).expect("handle");
        assert!(handle.is_alive());
        assert_eq!(
            handle
                .backend()
                .upgrade()
                .map(|backend| backend.display_id()),
            Some(reader.connection().backend().display_id())
        );
    }
}

#[test]
fn an_unknown_or_empty_identifier_has_no_handle() {
    let (_compositor, reader) = reader();
    assert!(reader.ext_handle("c3").is_none());
    assert!(reader.ext_handle("").is_none());
}
