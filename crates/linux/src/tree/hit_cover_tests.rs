use super::*;

pub(crate) fn client(id: u32, pid: u32, title: &str) -> X11Client {
    X11Client {
        shadow: (0, 0, 0, 0),
        id,
        hidden: false,
        pid: Some(pid),
        title: title.to_owned(),
        extents: None,
        bounds: None,
    }
}

#[test]
fn a_cover_that_is_the_only_client_of_its_process_and_title_is_named() {
    let clients = [client(1, 5, "Cover"), client(2, 5, "Other")];
    assert_eq!(cover_name(&clients, 1).as_deref(), Some("Cover"));
}

#[test]
fn a_cover_sharing_process_and_title_with_another_client_stays_unnamed() {
    let clients = [client(1, 5, "Cover"), client(2, 5, "Cover")];
    assert_eq!(cover_name(&clients, 1), None);
}

#[test]
fn the_same_title_in_another_process_does_not_make_the_cover_ambiguous() {
    let clients = [client(1, 5, "Cover"), client(2, 6, "Cover")];
    assert_eq!(cover_name(&clients, 1).as_deref(), Some("Cover"));
}

#[test]
fn an_untitled_or_unknown_cover_stays_unnamed() {
    let clients = [client(1, 5, "")];
    assert_eq!(cover_name(&clients, 1), None);
    assert_eq!(cover_name(&clients, 9), None);
}

#[test]
fn an_override_redirect_cover_with_a_unique_title_is_named() {
    let names = [(7, "Popup".to_owned())];
    assert_eq!(override_name(&names, &[], 7).as_deref(), Some("Popup"));
    assert_eq!(override_name(&names, &[], 8), None);
}

#[test]
fn an_override_redirect_cover_sharing_its_title_stays_unnamed() {
    let names = [(7, "Popup".to_owned())];
    assert_eq!(override_name(&names, &[client(1, 5, "Popup")], 7), None);
    let twins = [(7, "Popup".to_owned()), (8, "Popup".to_owned())];
    assert_eq!(override_name(&twins, &[], 7), None);
}
