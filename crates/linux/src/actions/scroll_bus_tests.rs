use super::every_window_hidden;
use crate::system::x11::X11Client;

fn client(pid: u32, hidden: bool) -> X11Client {
    X11Client {
        shadow: (0, 0, 0, 0),
        id: 1,
        pid: Some(pid),
        title: String::new(),
        extents: None,
        bounds: None,
        hidden,
    }
}

#[test]
fn an_app_whose_every_window_is_hidden_is_hidden() {
    assert!(every_window_hidden(&[client(7, true), client(7, true)], 7));
}

#[test]
fn one_visible_window_or_no_window_of_the_app_is_not_hidden() {
    assert!(!every_window_hidden(
        &[client(7, true), client(7, false)],
        7
    ));
    assert!(!every_window_hidden(&[client(8, true)], 7));
    assert!(!every_window_hidden(&[], 7));
}
