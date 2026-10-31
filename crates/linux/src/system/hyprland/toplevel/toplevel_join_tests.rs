use super::*;

fn record(identifier: &str, address: Option<u64>) -> (String, Option<u64>) {
    (identifier.to_owned(), address)
}

fn view(identifier: &str, address: u64) -> ToplevelView {
    ToplevelView {
        identifier: identifier.into(),
        address,
    }
}

#[test]
fn every_placed_toplevel_keeps_its_identifier_and_address_in_announce_order() {
    let views = join(&[record("b2", Some(0x20)), record("a1", Some(0x10))]).expect("a join");

    assert_eq!(views, vec![view("b2", 0x20), view("a1", 0x10)]);
}

#[test]
fn a_toplevel_the_mapping_manager_could_not_place_is_left_out() {
    let views = join(&[
        record("a1", None),
        record("", Some(0x20)),
        record("c3", Some(0x30)),
    ])
    .expect("a join");

    assert_eq!(views, vec![view("c3", 0x30)]);
}

#[test]
fn two_handles_for_one_address_make_the_read_unusable() {
    assert!(join(&[record("a1", Some(0x10)), record("b2", Some(0x10))]).is_none());
}

#[test]
fn an_unplaced_handle_never_counts_as_a_duplicate() {
    let views = join(&[
        record("a1", Some(0x10)),
        record("b2", None),
        record("c3", None),
    ]);

    assert_eq!(views.map(|views| views.len()), Some(1));
}
