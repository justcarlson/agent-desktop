use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;
use crate::input::hyprland::bound_geometry::unplaced_as;
use crate::tree::vocabulary::role_numbers::{MENU, PANEL};

const FRAME: u32 = 23;

fn cause(error: &AdapterError) -> &str {
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .unwrap_or_default()
}

#[test]
fn a_node_inside_a_menu_popup_is_refused_before_any_event() {
    let refusal = popup_surface_refusal(Some(MENU), false).expect("a menu frame is a popup");
    assert_eq!(refusal.code, ErrorCode::ActionFailed);
    assert_eq!(cause(&refusal), "popup_surface");
    assert_eq!(refusal.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_node_inside_a_popup_surface_has_no_bounds() {
    let refusal = popup_surface_refusal(Some(MENU), false).expect("a menu frame is a popup");
    let bounds: Option<u8> = unplaced_as(Err(refusal), None).expect("not a timeout");
    assert_eq!(bounds, None);
}

#[test]
fn a_panel_frame_is_a_popup_only_below_a_menu_target() {
    assert!(popup_surface_refusal(Some(PANEL), true).is_some());
    assert!(popup_surface_refusal(Some(PANEL), false).is_none());
}

#[test]
fn a_toplevel_frame_keeps_its_compositor_placement() {
    assert!(popup_surface_refusal(Some(FRAME), true).is_none());
    assert!(popup_surface_refusal(None, true).is_none());
}
