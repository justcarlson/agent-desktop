use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, WindowInfo, WindowState};

use super::menu_popup::Node;
use super::{MAX_POLLS, MenuPopup, PopupReader, entries_across, entry, find_popups};
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

fn owner(focused: bool) -> WindowInfo {
    WindowInfo {
        id: "w-1-2".to_owned(),
        title: "App".to_owned(),
        app: "app".to_owned(),
        pid: 7.into(),
        process_instance: Some("i".to_owned()),
        bounds: None,
        state: WindowState {
            is_focused: focused,
            ..WindowState::default()
        },
    }
}

fn popups(window: Node) -> Vec<MenuPopup> {
    find_popups(&mut Fake::new(window), false, deadline()).expect("a scan")
}

#[test]
fn a_closed_menu_lists_no_entry_though_its_nodes_are_present() {
    let closed = popups(frame_with(vec![qt_bar(false), gtk3_bar(false)]));
    assert!(closed.is_empty());
}

#[test]
fn an_open_menu_lists_one_menu_entry_on_the_owner_window_with_its_item_count() {
    let open = popups(frame_with(vec![gtk4_popover(true)]));
    let entry = entry(&open[0], &owner(false));
    assert_eq!(entry.id, "w-1-2");
    assert_eq!(entry.kind, "menu");
    assert_eq!(entry.item_count, Some(2));
    assert!(entry.unclassified.is_empty());
}

#[test]
fn the_first_popup_of_a_chain_is_the_outermost_and_the_entry_counts_it() {
    let inner_items = vec![
        item("A", true),
        item("B", true),
        item("C", true),
        item("D", true),
        item("E", true),
    ];
    let inner = node(MENU, "More", &[SHOWING], inner_items);
    let more = node(ITEM, "More", &[SHOWING], vec![inner]);
    let outer = node(MENU, "File", &[SHOWING], vec![item("Open", true), more]);
    let found = popups(frame_with(vec![node(
        MENU_BAR,
        "",
        &[SHOWING],
        vec![outer],
    )]));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].items, 2);
    assert_eq!(found[1].items, 5);
    assert_eq!(entry(&found[0], &owner(true)).item_count, Some(2));
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

fn window_named(id: &str) -> WindowInfo {
    WindowInfo {
        id: id.to_owned(),
        ..owner(false)
    }
}

#[test]
fn a_top_level_popup_is_listed_once_on_the_first_window_that_sees_it() {
    let (first, second) = (window_named("w-1-1"), window_named("w-1-3"));
    let listed = entries_across(&[&first, &second], |_| {
        find_popups(
            &mut Fake {
                top: vec![chromium_frame()],
                ..Fake::new(frame_with(Vec::new()))
            },
            false,
            deadline(),
        )
    })
    .expect("entries");
    let ids: Vec<&str> = listed.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, ["w-1-1"]);
}

#[test]
fn an_in_window_popup_is_listed_on_each_window_that_holds_one() {
    let (first, second) = (window_named("w-1-1"), window_named("w-1-3"));
    let listed = entries_across(&[&first, &second], |_| {
        find_popups(
            &mut Fake::new(frame_with(vec![gtk4_popover(true)])),
            false,
            deadline(),
        )
    })
    .expect("entries");
    assert_eq!(listed.len(), 2);
}

#[test]
fn a_window_that_sees_no_popup_does_not_claim_a_top_level_one() {
    let (first, second) = (window_named("w-1-1"), window_named("w-1-3"));
    let mut calls = 0;
    let listed = entries_across(&[&first, &second], |_| {
        calls += 1;
        let mut fake = Fake::new(frame_with(Vec::new()));
        if calls == 2 {
            fake.top = vec![gtk3_context_window()];
        }
        find_popups(&mut fake, false, deadline())
    })
    .expect("entries");
    assert_eq!(listed[0].id, "w-1-3");
}
