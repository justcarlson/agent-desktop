use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

fn client_at(x: f64, y: f64) -> HyprClient {
    HyprClient {
        address: 0xa1,
        pid: 4100,
        title: "Calculator".to_owned(),
        class: "org.gnome.Calculator".to_owned(),
        geometry: Rect {
            x,
            y,
            width: 360.0,
            height: 616.0,
        },
        workspace: ClientWorkspace {
            name: "1".to_owned(),
            monitor: Some(0),
        },
        shown: true,
    }
}

#[test]
fn the_origin_is_the_client_position_in_the_layout() {
    assert_eq!(origin_of(&client_at(1932.0, -1042.0)), Some((1932, -1042)));
}

#[test]
fn a_fractional_position_rounds_to_the_nearest_pixel() {
    assert_eq!(origin_of(&client_at(12.6, 38.4)), Some((13, 38)));
}

#[test]
fn a_position_that_is_not_finite_or_out_of_range_has_no_origin() {
    assert_eq!(origin_of(&client_at(f64::NAN, 0.0)), None);
    assert_eq!(origin_of(&client_at(0.0, 1e12)), None);
}
