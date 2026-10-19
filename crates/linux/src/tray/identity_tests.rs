use super::*;

fn item() -> TrayTarget {
    TrayTarget {
        owner: ":1.55".into(),
        object_path: "/org/ayatana/NotificationItem/l21".into(),
        menu_path: None,
        entry_id: None,
        revision: None,
        label_path: Vec::new(),
    }
}

fn entry(labels: &[&str]) -> TrayTarget {
    TrayTarget {
        menu_path: Some("/MenuBar".into()),
        entry_id: Some(7),
        label_path: labels.iter().map(|label| (*label).into()).collect(),
        ..item()
    }
}

#[test]
fn an_item_round_trips_through_its_persisted_form() {
    let target = item();
    assert_eq!(decode(&encode(&target)), Some(target));
}

#[test]
fn an_entry_round_trips_with_separators_inside_its_labels() {
    let target = entry(&["File|Open", "100% / done", ""]);
    let text = encode(&target);
    assert_eq!(text.matches('|').count(), 5, "{text}");
    assert_eq!(decode(&text), Some(target));
}

#[test]
fn the_revision_is_not_part_of_the_persisted_identity() {
    let mut target = entry(&["Open"]);
    let without = encode(&target);
    target.revision = Some(9);
    assert_eq!(encode(&target), without);
    assert_eq!(decode(&without).and_then(|decoded| decoded.revision), None);
}

#[test]
fn an_item_and_an_entry_of_the_same_item_have_distinct_identities() {
    assert_ne!(encode(&item()), encode(&entry(&["Open"])));
}

#[test]
fn malformed_text_decodes_to_nothing() {
    for text in [
        "",
        "tray:v1:",
        "tray:v2:item|:1.5|/p",
        "tray:v1:item|:1.5",
        "tray:v1:item||/p",
        "tray:v1:item|:1.5|no-slash",
        "tray:v1:entry|:1.5|/p|/m|notanumber|Open",
        "tray:v1:entry|:1.5|/p|/m|3|bad%ZZ",
        "tray:v1:menu|:1.5|/p",
        "/org/a11y/atspi/accessible/12",
    ] {
        assert_eq!(decode(text), None, "{text}");
    }
}

#[test]
fn ownership_follows_the_prefix_alone() {
    assert!(is_tray_id("tray:v1:garbage"));
    assert!(!is_tray_id("/org/a11y/atspi/accessible/12"));
}
