use super::*;
use crate::notifications::fake::{reading, spec};

fn first_entry(specs: &[crate::notifications::fake::Spec]) -> Entry {
    let read = reading(specs, false);
    let (subtrees, _) = partition(&read.nodes);
    extract(subtrees[0], 1).expect("an entry")
}

#[test]
fn open_and_closed_shapes_give_the_same_fields() {
    let closed = first_entry(&[spec("Mail", "Hello").body("World")]);
    let open = first_entry(&[spec("Mail", "Hello").body("World").time("5 minutes ago")]);
    assert_eq!(closed.info.app_name, open.info.app_name);
    assert_eq!(closed.info.title, open.info.title);
    assert_eq!(closed.info.body, open.info.body);
    assert_eq!(open.info.title, "Hello");
}

#[test]
fn an_empty_body_is_none_and_a_long_body_is_whole() {
    assert_eq!(first_entry(&[spec("Mail", "Hi")]).info.body, None);
    let long: &'static str = Box::leak("x".repeat(700).into_boxed_str());
    let entry = first_entry(&[spec("Mail", "Hi").body(long)]);
    assert_eq!(entry.info.body.as_deref().map(str::len), Some(700));
}

#[test]
fn rtl_and_emoji_text_survive() {
    let entry = first_entry(&[spec("البريد", "مرحبا 🎉").body("שלום")]);
    assert_eq!(entry.info.app_name, "البريد");
    assert_eq!(entry.info.title, "مرحبا 🎉");
    assert_eq!(entry.info.body.as_deref(), Some("שלום"));
}

#[test]
fn actions_are_the_named_buttons_and_their_labels_do_not_leak_into_the_text() {
    let entry = first_entry(&[spec("Mail", "Hi").body("b").actions(&["Run A", "Run B"])]);
    assert_eq!(entry.info.actions, vec!["Run A", "Run B"]);
    assert_eq!(entry.info.body.as_deref(), Some("b"));
}

#[test]
fn a_two_label_entry_keeps_a_time_shaped_title() {
    let entry = first_entry(&[spec("Clock", "Just now")]);
    assert_eq!(entry.info.title, "Just now");
}

#[test]
fn an_entry_with_no_title_is_none() {
    let read = reading(&[spec("Mail", "")], false);
    let (subtrees, _) = partition(&read.nodes);
    assert!(extract(subtrees[0], 1).is_none());
}

#[test]
fn time_labels_are_told_by_shape() {
    for text in [
        "Just now",
        "now",
        "Yesterday",
        "5 minutes ago",
        "1 hour ago",
        "3 hrs ago",
        "12:05",
        "9:30 PM",
        "9:30pm",
    ] {
        assert!(is_time_label(text), "{text}");
    }
    for text in ["Meeting at noon", "5 apples", "ago", "Build 12:5", "Hello"] {
        assert!(!is_time_label(text), "{text}");
    }
}

#[test]
fn laid_out_rejects_sentinels_and_empty_rectangles() {
    let good = Extents {
        x: 1,
        y: 1,
        width: 5,
        height: 5,
    };
    assert!(is_laid_out(Some(good)));
    assert!(!is_laid_out(None));
    assert!(!is_laid_out(Some(Extents {
        x: i32::MIN,
        y: i32::MIN,
        width: -1,
        height: -1
    })));
    assert!(!is_laid_out(Some(Extents { width: 0, ..good })));
}
