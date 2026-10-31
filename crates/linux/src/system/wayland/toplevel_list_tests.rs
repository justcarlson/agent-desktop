use agent_desktop_core::Deadline;

use super::*;
use crate::system::wayland::fake_compositor::{FakeCompositor, Script};
use crate::system::wayland::registry_read::read_registry;
use crate::system::wayland::socket_connect::connect;

fn advertised(globals: Vec<(&'static str, u32)>) -> Option<Global> {
    let compositor = FakeCompositor::start(Script::Answer {
        globals,
        outputs: vec![],
    });
    let stream = connect(&compositor.socket).expect("connects");
    let deadline = Deadline::after(2_000).expect("deadline");
    let exchange = read_registry(stream, deadline).expect("answers");
    advertised_list(&exchange).cloned()
}

#[test]
fn the_list_is_found_from_its_registry_entry_on_any_compositor() {
    let list = advertised(vec![("wl_compositor", 6), (LIST_INTERFACE, 1)]).expect("advertised");
    assert_eq!(list.interface, LIST_INTERFACE);
}

#[test]
fn a_compositor_without_the_list_has_none_whatever_else_it_advertises() {
    assert_eq!(
        advertised(vec![
            ("wl_compositor", 6),
            ("hyprland_toplevel_mapping_manager_v1", 1),
        ]),
        None
    );
}
