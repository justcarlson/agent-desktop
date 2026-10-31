use super::*;

const CLIENTS: &str = r#"[
  {"address":"0x55e7761d2e70","mapped":true,"hidden":false,"at":[10,40],"size":[621,750],
   "workspace":{"id":1,"name":"1"},"monitor":0,"floating":false,"pid":3845,"class":"two_windows.py",
   "title":"ADTwin","xwayland":false},
  {"address":"0x55e776012c90","mapped":true,"hidden":false,"at":[640,40],"size":[300,200],
   "workspace":{"id":-98,"name":"special:hid"},"floating":true,"pid":2307,"class":"Alacritty",
   "title":"ad-alacritty","xwayland":false},
  {"address":"0x1","mapped":false,"hidden":false,"at":[0,0],"size":[0,0],
   "workspace":{"id":-1,"name":""},"pid":9,"class":"","title":""}
]"#;

#[test]
fn every_client_keeps_its_address_process_title_geometry_and_workspace() {
    let clients = parse_clients(CLIENTS.as_bytes()).expect("a client list");

    assert_eq!(clients.len(), 3);
    let twin = &clients[0];
    assert_eq!(twin.address, 0x55e7761d2e70);
    assert_eq!(twin.pid, 3845);
    assert_eq!(twin.title, "ADTwin");
    assert_eq!(twin.class, "two_windows.py");
    assert_eq!((twin.geometry.x, twin.geometry.y), (10.0, 40.0));
    assert_eq!((twin.geometry.width, twin.geometry.height), (621.0, 750.0));
    assert_eq!(twin.workspace.name, "1");
    assert_eq!(twin.workspace.monitor, Some(0));
    assert!(twin.shown);
    assert_eq!(clients[1].workspace.name, "special:hid");
    assert_eq!(clients[1].workspace.monitor, None);
}

#[test]
fn an_unmapped_client_is_kept_but_not_shown() {
    let clients = parse_clients(CLIENTS.as_bytes()).expect("a client list");

    assert!(!clients[2].shown);
}

#[test]
fn a_client_without_a_readable_address_or_pid_makes_the_reply_unusable() {
    let missing_pid = r#"[{"address":"0x2","mapped":true,"hidden":false,"at":[0,0],
        "size":[1,1],"workspace":{"id":1,"name":"1"},"class":"a","title":"b"}]"#;
    let bad_address = r#"[{"address":"zz","mapped":true,"hidden":false,"at":[0,0],
        "size":[1,1],"workspace":{"id":1,"name":"1"},"pid":4,"class":"a","title":"b"}]"#;

    assert!(parse_clients(missing_pid.as_bytes()).is_none());
    assert!(parse_clients(bad_address.as_bytes()).is_none());
    assert!(parse_clients(b"not json").is_none());
}

#[test]
fn the_active_window_is_its_address_and_an_empty_object_is_none() {
    assert_eq!(
        parse_active(br#"{"address":"0x55e776012c90","pid":2307}"#),
        Some(Some(0x55e776012c90))
    );
    assert_eq!(parse_active(b"{}"), Some(None));
    assert_eq!(parse_active(b"[]"), None);
}

#[test]
fn an_address_is_read_as_hexadecimal_with_its_prefix() {
    assert_eq!(parse_address("0x10"), Some(16));
    assert_eq!(parse_address("10"), None);
    assert_eq!(parse_address("0x"), None);
}
