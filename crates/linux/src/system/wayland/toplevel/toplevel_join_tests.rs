use super::*;

fn ext(identifier: &str, address: Option<u64>) -> ExtRecord {
    ExtRecord {
        identifier: identifier.into(),
        address,
        answered: true,
    }
}

fn wlr(address: Option<u64>, activated: bool) -> WlrRecord {
    WlrRecord {
        address,
        answered: true,
        activated,
        closed: false,
    }
}

#[test]
fn a_toplevel_joins_its_ext_identifier_to_the_wlr_handle_with_the_same_address() {
    let views = join(
        &[ext("a1", Some(0x10)), ext("b2", Some(0x20))],
        &[wlr(Some(0x20), true), wlr(Some(0x10), false)],
    )
    .expect("a join");

    assert_eq!(
        views,
        vec![
            ToplevelView {
                identifier: "a1".into(),
                address: 0x10,
                activated: false,
                wlr_index: Some(1),
            },
            ToplevelView {
                identifier: "b2".into(),
                address: 0x20,
                activated: true,
                wlr_index: Some(0),
            },
        ]
    );
}

#[test]
fn a_toplevel_the_mapping_manager_could_not_place_is_left_out() {
    let views = join(
        &[ext("a1", None), ext("", Some(0x20))],
        &[wlr(Some(0x20), true)],
    )
    .expect("a join");

    assert!(views.is_empty());
}

#[test]
fn a_toplevel_without_a_wlr_handle_is_listed_but_cannot_be_activated() {
    let views = join(&[ext("a1", Some(0x10))], &[]).expect("a join");

    assert_eq!(views[0].wlr_index, None);
    assert!(!views[0].activated);
}

#[test]
fn a_closed_wlr_handle_is_not_a_handle_for_its_address() {
    let mut closed = wlr(Some(0x10), true);
    closed.closed = true;

    let views = join(&[ext("a1", Some(0x10))], &[closed]).expect("a join");

    assert_eq!(views[0].wlr_index, None);
}

#[test]
fn two_handles_of_one_protocol_for_one_address_make_the_read_unusable() {
    assert!(join(&[ext("a1", Some(0x10)), ext("b2", Some(0x10))], &[]).is_none());
    assert!(
        join(
            &[ext("a1", Some(0x10))],
            &[wlr(Some(0x10), true), wlr(Some(0x10), false)]
        )
        .is_none()
    );
}
