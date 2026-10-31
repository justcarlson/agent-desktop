use agent_desktop_core::Deadline;

use super::*;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::socket_connect::connect;

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

#[test]
fn the_registry_lists_what_the_compositor_advertises() {
    let compositor = FakeCompositor::start(Script::Answer {
        globals: vec![("wl_compositor", 6), ("ext_foreign_toplevel_list_v1", 1)],
        outputs: vec![],
    });
    let stream = connect(&compositor.socket).expect("connects");
    let exchange = read_registry(stream, deadline(2_000)).expect("answers");
    assert!(exchange.find("ext_foreign_toplevel_list_v1", 1).is_some());
    assert!(
        exchange
            .find("hyprland_toplevel_mapping_manager_v1", 1)
            .is_none()
    );
}

#[test]
fn a_silent_compositor_ends_the_read_as_silent() {
    let compositor = FakeCompositor::start(Script::Silent);
    let stream = connect(&compositor.socket).expect("connects");
    assert_eq!(
        read_registry(stream, deadline(200)).err(),
        Some(WaylandFault::Silent)
    );
}
