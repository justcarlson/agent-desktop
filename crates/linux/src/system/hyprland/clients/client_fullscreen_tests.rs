use super::*;

const CLIENTS: &str = r#"[
  {"address":"0x10","workspace":{"id":1,"name":"1"},"fullscreen":0},
  {"address":"0x20","workspace":{"id":1,"name":"1"},"fullscreen":2},
  {"address":"0x30","workspace":{"id":2,"name":"2"},"fullscreen":1}
]"#;

#[test]
fn fullscreen_and_maximized_clients_fill_their_workspace() {
    assert_eq!(
        parse_filling(CLIENTS.as_bytes()),
        Some(vec![(0x20, "1".into()), (0x30, "2".into())])
    );
}

#[test]
fn no_client_filling_its_workspace_is_an_empty_list() {
    let none = r#"[{"address":"0x10","workspace":{"id":1,"name":"1"},"fullscreen":0}]"#;

    assert_eq!(parse_filling(none.as_bytes()), Some(Vec::new()));
}

#[test]
fn a_client_without_a_readable_fullscreen_mode_makes_the_reply_unusable() {
    let missing = r#"[{"address":"0x10","workspace":{"id":1,"name":"1"}}]"#;
    let boolean = r#"[{"address":"0x10","workspace":{"id":1,"name":"1"},"fullscreen":true}]"#;

    assert_eq!(parse_filling(missing.as_bytes()), None);
    assert_eq!(parse_filling(boolean.as_bytes()), None);
    assert_eq!(parse_filling(b"not json"), None);
}
