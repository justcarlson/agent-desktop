use crate::system::shell::fake_tree::{SENTINEL, at};
use crate::system::shell::live_state::project;
use crate::tree::Extents;
use crate::tree::display_bounds::off_every_display;
use crate::tree::offscreen::{OffscreenInput, offscreen};
use crate::tree::placement::Placement;

fn displays() -> Vec<Extents> {
    vec![at(0, 0, 1280, 800)]
}

fn verdict(placement: &Placement) -> Option<bool> {
    let hidden = placement.x11.is_some_and(|state| state.hidden);
    let minimized = if hidden {
        Some(true)
    } else {
        placement.x11.map(|_| false)
    };
    let by_rule = offscreen(OffscreenInput {
        node: placement.node,
        frame: placement.frame,
        container: None,
        visible: true,
        showing: true,
        sentinel: false,
        minimized,
    });
    let off_screen =
        by_rule != Some(true) && off_every_display(placement.node, placement.displays.as_deref());
    if off_screen { Some(true) } else { by_rule }
}

#[test]
fn top_bar_and_dock_nodes_outside_the_content_window_are_not_offscreen() {
    for node in [at(0, 0, 72, 32), at(1204, 0, 76, 32), at(534, 704, 192, 60)] {
        let placement = project(Some(node), Some(&displays()));
        assert_eq!(verdict(&placement), Some(false), "{node:?}");
    }
}

#[test]
fn the_content_window_rectangle_is_not_the_frame_the_node_must_lie_in() {
    let placement = project(Some(at(1204, 0, 76, 32)), Some(&displays()));
    assert_eq!(placement.frame, Some(at(0, 0, 1280, 800)));
    assert_ne!(placement.frame, Some(at(0, 54, 87, 55)));
}

#[test]
fn extents_wholly_off_every_display_are_offscreen() {
    let placement = project(Some(at(-84, 549, 84, 60)), Some(&displays()));
    assert_eq!(verdict(&placement), Some(true));
}

#[test]
fn the_sentinel_position_is_offscreen() {
    let placement = project(Some(SENTINEL), Some(&displays()));
    assert_eq!(verdict(&placement), Some(true));
}

#[test]
fn a_merely_negative_origin_that_overlaps_a_display_is_not_offscreen() {
    let placement = project(Some(at(0, -1, 60, 84)), Some(&displays()));
    assert_eq!(verdict(&placement), Some(false));
}

#[test]
fn a_node_with_no_area_at_a_real_position_is_not_called_offscreen() {
    let placement = project(Some(at(0, 0, 0, 0)), Some(&displays()));
    assert_ne!(verdict(&placement), Some(true));
}

#[test]
fn unreadable_displays_prove_nothing_either_way() {
    let placement = project(Some(at(0, 0, 72, 32)), None);
    assert_eq!(placement.frame, None);
    assert_eq!(verdict(&placement), None);
    let none = project(None, Some(&displays()));
    assert_eq!(verdict(&none), None);
}

#[test]
fn the_client_origin_is_zero_because_window_and_screen_coordinates_agree() {
    let placement = project(Some(at(0, 0, 72, 32)), Some(&displays()));
    assert_eq!(placement.x11.map(|state| state.client_origin), Some((0, 0)));
    assert!(!placement.frame_iconified);
}
