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
fn one_unreadable_client_makes_the_reply_unusable() {
    let reply = r#"[{"address": "0x1", "at": [0, 0], "size": [1, 1], "floating": true,
        "workspace": {"name": "1"}}]"#;

    assert_eq!(parse_states(reply.as_bytes()), None);
    assert_eq!(parse_states(b"unknown request"), None);
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
