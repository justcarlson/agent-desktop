use super::*;

#[test]
fn every_monitor_is_read_with_its_id_and_layout_origin() {
    let body = br#"[{"id": 0, "name": "WAYLAND-1", "x": -1024, "y": -640, "scale": 1.25},
                    {"id": 3, "name": "HDMI-A-1", "x": 1280, "y": 0, "scale": 1.0}]"#;

    let origins = parse_origins(body).expect("readable");

    assert_eq!(
        origins,
        vec![
            OutputOrigin {
                id: 0,
                at: (-1024, -640)
            },
            OutputOrigin {
                id: 3,
                at: (1280, 0)
            },
        ]
    );
    assert_eq!(origin_of(&origins, Some(3)), Some((1280, 0)));
    assert_eq!(origin_of(&origins, Some(7)), None);
    assert_eq!(origin_of(&origins, None), None);
}

#[test]
fn a_monitor_without_its_origin_makes_the_reply_unreadable() {
    assert_eq!(parse_origins(br#"[{"id": 0, "x": 0}]"#), None);
    assert_eq!(parse_origins(b"not json"), None);
}
