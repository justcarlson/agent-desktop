use super::*;
use crate::notifications::fake::{FakeWorld, lease, log, reading, spec};
use crate::notifications::list::read_listing;
use agent_desktop_core::InteractionPolicy;

fn first_entry_and_list(world: &FakeWorld) -> (Entry, Vec<Seen>) {
    let listing = read_listing(world, lease(600).deadline()).expect("listing");
    let before = listing.seen();
    (
        listing.entries.into_iter().next().expect("an entry"),
        before,
    )
}

fn close(world: &FakeWorld, entry: &Entry, before: &[Seen]) -> Result<Vec<Seen>, Unclosed> {
    close_entry(
        world,
        entry,
        before,
        InteractionPolicy::headed(),
        &lease(600),
    )
}

#[test]
fn an_entry_with_no_header_button_is_not_pressed_at_all() {
    let world = FakeWorld::new(&log(), vec![reading(&[spec("Mail", "a")])]);
    let (mut entry, before) = first_entry_and_list(&world);
    entry.header.clear();
    assert!(matches!(
        close(&world, &entry, &before),
        Err(Unclosed::Press(_))
    ));
    assert!(world.clicks().is_empty());
}

#[test]
fn a_bystander_leaving_with_the_target_is_not_a_clean_close() {
    let world = FakeWorld::new(
        &log(),
        vec![
            reading(&[spec("Mail", "a"), spec("Chat", "b")]),
            reading(&[]),
        ],
    );
    let (entry, before) = first_entry_and_list(&world);
    assert!(matches!(
        close(&world, &entry, &before),
        Err(Unclosed::Bystander)
    ));
}

#[test]
fn an_entry_that_stays_after_every_header_button_is_a_failure() {
    let world = FakeWorld::new(&log(), vec![reading(&[spec("Mail", "a")])]);
    let (entry, before) = first_entry_and_list(&world);
    assert!(matches!(
        close(&world, &entry, &before),
        Err(Unclosed::Survived | Unclosed::Unread(_))
    ));
    assert_eq!(world.clicks(), vec!["/Mail:a/close", "/Mail:a/expand"]);
}
