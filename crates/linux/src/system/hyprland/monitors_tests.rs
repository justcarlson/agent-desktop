use super::*;

fn monitor(
    id: i64,
    name: &str,
    geometry: (i64, i64, i64, i64),
    scale: f64,
    transform: u64,
) -> Value {
    let (x, y, width, height) = geometry;
    serde_json::json!({
        "id": id, "name": name, "description": "", "width": width, "height": height,
        "refreshRate": 60.0, "x": x, "y": y, "scale": scale, "transform": transform,
        "focused": id == 1, "disabled": false
    })
}

fn parsed(monitors: Vec<Value>) -> Vec<DisplayInfo> {
    parse(&serde_json::to_vec(&monitors).expect("json")).expect("parses")
}

#[test]
fn the_lowest_id_is_primary_and_comes_first_whatever_has_focus() {
    let displays = parsed(vec![
        monitor(1, "HDMI-A-1", (1920, 0, 1920, 1080), 1.0, 0),
        monitor(0, "eDP-1", (0, 0, 1920, 1080), 1.0, 0),
    ]);
    assert_eq!(
        displays
            .iter()
            .map(|display| (display.id.as_str(), display.is_primary))
            .collect::<Vec<_>>(),
        vec![("eDP-1", true), ("HDMI-A-1", false)]
    );
}

#[test]
fn the_logical_size_is_the_mode_divided_by_the_scale() {
    let displays = parsed(vec![monitor(0, "eDP-1", (0, 0, 2880, 1800), 2.0, 0)]);
    assert_eq!(
        displays[0].bounds,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0
        }
    );
    assert_eq!(displays[0].scale, 2.0);
}

#[test]
fn a_quarter_turn_swaps_width_and_height_and_a_half_turn_does_not() {
    let displays = parsed(vec![
        monitor(0, "DP-1", (0, 0, 1920, 1080), 1.0, 1),
        monitor(1, "DP-2", (1080, 0, 1920, 1080), 1.0, 2),
    ]);
    assert_eq!(
        (displays[0].bounds.width, displays[0].bounds.height),
        (1080.0, 1920.0)
    );
    assert_eq!(
        (displays[1].bounds.width, displays[1].bounds.height),
        (1920.0, 1080.0)
    );
}

#[test]
fn a_reply_that_is_not_a_monitor_list_is_unreadable() {
    assert_eq!(parse(b"ok"), None);
    assert_eq!(parse(br#"{"id":0}"#), None);
    assert_eq!(parse(br#"[{"id":0,"name":"x"}]"#), None);
    assert_eq!(
        parse(&serde_json::to_vec(&vec![monitor(0, "x", (0, 0, 10, 10), 0.0, 0)]).expect("json")),
        None
    );
    assert_eq!(parse(b"[]"), Some(Vec::new()));
}
