use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::*;

const REPLY: &str = r#"[
  {"address": "0x55d0a1", "at": [40, 60], "size": [500, 400], "floating": true,
   "fullscreen": 1, "workspace": {"id": -98, "name": "special:agent-desktop-minimized-1"}},
  {"address": "0x77", "at": [0.4, 12.6], "size": [7, 17], "floating": false,
   "fullscreen": 0, "workspace": {"id": 1, "name": "1"}}
]"#;

#[test]
fn every_client_is_read_with_its_place_and_state() {
    let states = parse_states(REPLY.as_bytes()).expect("readable");

    assert_eq!(
        states,
        vec![
            ClientState {
                address: 0x55d0a1,
                at: (40, 60),
                size: (500, 400),
                floating: true,
                fullscreen: 1,
                workspace: "special:agent-desktop-minimized-1".into(),
            },
            ClientState {
                address: 0x77,
                at: (0, 13),
                size: (7, 17),
                floating: false,
                fullscreen: 0,
                workspace: "1".into(),
            },
        ]
    );
}

#[test]
fn one_unreadable_client_refuses_for_missing_state_evidence() {
    let reply = r#"[{"address": "0x1", "at": [0, 0], "size": [1, 1], "floating": true,
        "workspace": {"name": "1"}}]"#;

    for body in [reply.as_bytes(), b"unknown request".as_slice()] {
        let error = parse_states(body).expect_err("unreadable");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        let details = error.details.expect("details");
        assert_eq!(details["cause"], "window_state_unreadable");
        assert_eq!(details["physical_delivery_started"], false);
    }
}

#[test]
fn the_json_form_names_every_field() {
    let state = &parse_states(REPLY.as_bytes()).expect("readable")[0];

    assert_eq!(
        state.to_json(),
        serde_json::json!({"x": 40, "y": 60, "width": 500, "height": 400, "floating": true,
            "fullscreen": 1, "workspace": "special:agent-desktop-minimized-1"})
    );
}
