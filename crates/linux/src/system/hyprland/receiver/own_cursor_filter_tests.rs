use agent_desktop_core::Rect;

use super::OwnCursorFilter;
use crate::system::hyprland::cursor_renderer::hold_registry::{publish, withdraw};
use crate::system::hyprland::receiver::layer_info::LayerInfo;
use crate::system::hyprland::receiver::overlay_filter::OverlayFilter;

fn layer(namespace: &str, pid: Option<u32>) -> LayerInfo {
    LayerInfo {
        namespace: namespace.into(),
        pid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
        },
        level: 3,
        output: "DP-1".into(),
    }
}

#[test]
fn only_an_acknowledged_renderers_cursor_layer_passes() {
    let filter = OwnCursorFilter::of(vec![4_100_001]);
    assert!(filter.is_own_cursor(&layer("agent-desktop-cursor", Some(4_100_001))));
    assert!(!filter.is_own_cursor(&layer("agent-desktop-cursor", Some(4_100_002))));
    assert!(!filter.is_own_cursor(&layer("agent-desktop-cursor", None)));
    assert!(!filter.is_own_cursor(&layer("waybar", Some(4_100_001))));
}

#[test]
fn the_current_filter_follows_this_threads_hold() {
    let cursor = layer("agent-desktop-cursor", Some(4_100_003));
    assert!(!OwnCursorFilter::current().is_own_cursor(&cursor));
    publish("own-filter-a", vec![4_100_003]);
    assert!(OwnCursorFilter::current().is_own_cursor(&cursor));
    withdraw("own-filter-a");
    assert!(!OwnCursorFilter::current().is_own_cursor(&cursor));
}
