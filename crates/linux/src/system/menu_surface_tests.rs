use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};

use super::menu_popup::{MenuFamily, Node};
use super::{MAX_POLLS, MenuPopup, PopupReader, find_popups};
use crate::system::menu_fixture::*;

struct Fake {
    window: Node,
    top: Vec<Node>,
    appears_after: Option<usize>,
    scans: usize,
    pauses: usize,
}

impl Fake {
    fn new(window: Node) -> Self {
        Self {
            window,
            top: Vec::new(),
            appears_after: None,
            scans: 0,
            pauses: 0,
        }
    }

    fn opening_after(mut self, scans: usize) -> Self {
        self.appears_after = Some(scans);
        self
    }
}

impl PopupReader for Fake {
    fn window(&mut self) -> Result<Option<Node>, AdapterError> {
        self.scans += 1;
        let hidden = self.appears_after.is_some_and(|after| self.scans <= after);
        Ok(Some(if hidden {
            frame_with(Vec::new())
        } else {
            self.window.clone()
        }))
    }

    fn top_levels(&mut self) -> Result<Vec<Node>, AdapterError> {
        Ok(self.top.clone())
    }

    fn pause(&mut self, _gap: Duration) {
        self.pauses += 1;
    }
}

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline")
}

fn popups(window: Node) -> Vec<MenuPopup> {
    find_popups(&mut Fake::new(window), false, deadline()).expect("a scan")
}

#[test]
fn one_synthetic_read_per_family_yields_one_popup_of_that_family() {
    let cases = [
        (
            frame_with(vec![gtk4_popover(true)]),
            MenuFamily::Gtk4Popover,
        ),
        (frame_with(vec![gtk3_bar(true)]), MenuFamily::Gtk3Bar),
        (frame_with(vec![qt_bar(true)]), MenuFamily::Qt),
    ];
    for (window, family) in cases {
        let found = popups(window);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].family, family);
    }
    for (top, family) in [
        (gtk3_context_window(), MenuFamily::Gtk3Context),
        (chromium_frame(), MenuFamily::Chromium),
    ] {
        let mut fake = Fake::new(frame_with(Vec::new()));
        fake.top = vec![top];
        let found = find_popups(&mut fake, false, deadline()).expect("a scan");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].family, family);
    }
}

#[test]
fn a_popup_that_appears_after_the_settle_poll_is_listed() {
    let mut fake = Fake::new(frame_with(vec![gtk4_popover(true)])).opening_after(2);
    let found = find_popups(&mut fake, true, deadline()).expect("a scan");
    assert_eq!(found.len(), 1);
    assert_eq!(fake.pauses, 2);
}

#[test]
fn a_popup_that_shows_330_ms_after_the_action_is_still_listed() {
    let mut fake = Fake::new(frame_with(vec![gtk4_popover(true)])).opening_after(9);
    let found = find_popups(&mut fake, true, deadline()).expect("a scan");
    assert_eq!(found.len(), 1);
    assert_eq!(fake.pauses, 9);
}

#[test]
fn the_settle_ends_without_an_entry_or_an_error_when_nothing_opens() {
    let mut fake = Fake::new(frame_with(Vec::new()));
    let found = find_popups(&mut fake, true, deadline()).expect("no error");
    assert!(found.is_empty());
    assert!(fake.pauses <= MAX_POLLS);
    assert!(fake.pauses > 0);
}

#[test]
fn an_unfocused_window_does_not_wait_for_a_popup() {
    let mut fake = Fake::new(frame_with(vec![gtk4_popover(true)])).opening_after(1);
    let found = find_popups(&mut fake, false, deadline()).expect("a scan");
    assert!(found.is_empty());
    assert_eq!(fake.pauses, 0);
}

#[test]
fn the_settle_stops_at_the_deadline() {
    let mut fake = Fake::new(frame_with(Vec::new()));
    let spent = Deadline::after(1).expect("a deadline");
    std::thread::sleep(Duration::from_millis(5));
    find_popups(&mut fake, true, spent).expect("no error");
    assert_eq!(fake.pauses, 0);
}

#[test]
fn a_popup_window_listed_as_a_window_does_not_list_its_own_menu() {
    let found = popups(gtk3_context_window());
    assert!(found.is_empty());
}
