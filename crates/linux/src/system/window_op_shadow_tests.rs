use super::*;
use serde_json::json;

const SHADOW: Shadow = (26, 23, 26, 29);

fn move_resize(
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
) -> WindowWrite {
    WindowWrite::MoveResize {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn a_move_asks_the_client_at_the_visible_place_less_the_shadow() {
    assert_eq!(
        to_client(move_resize(Some(300), Some(200), None, None), SHADOW),
        move_resize(Some(274), Some(177), None, None)
    );
}

#[test]
fn a_resize_asks_the_client_for_the_visible_size_plus_the_shadow() {
    assert_eq!(
        to_client(move_resize(None, None, Some(814), Some(577)), SHADOW),
        move_resize(None, None, Some(866), Some(629))
    );
}

#[test]
fn a_window_without_a_shadow_and_a_state_write_are_unchanged() {
    let write = move_resize(Some(1), Some(2), Some(3), Some(4));
    assert_eq!(to_client(write.clone(), (0, 0, 0, 0)), write);
    assert_eq!(
        to_client(WindowWrite::Iconify, SHADOW),
        WindowWrite::Iconify
    );
}

#[test]
fn an_error_reports_the_rectangles_as_the_visible_window() {
    let error = AdapterError::new(agent_desktop_core::ErrorCode::ActionFailed, "left other")
        .with_details(json!({
            "requested": { "x": null, "y": null, "width": 752, "height": 502 },
            "observed": { "x": 274, "y": 177, "width": 696, "height": 431, "hidden": false },
        }));
    let details = to_visible(error, SHADOW).details.expect("details");
    assert_eq!(details["requested"]["width"], 700);
    assert_eq!(details["requested"]["x"], serde_json::Value::Null);
    assert_eq!(details["observed"]["x"], 300);
    assert_eq!(details["observed"]["y"], 200);
    assert_eq!(details["observed"]["width"], 644);
    assert_eq!(details["observed"]["height"], 379);
}
