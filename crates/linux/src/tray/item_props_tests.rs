use zbus::zvariant::ObjectPath;

use super::*;

fn owned(value: Value<'_>) -> OwnedValue {
    OwnedValue::try_from(value).expect("a plain value converts")
}

fn map(pairs: Vec<(&str, Value<'_>)>) -> HashMap<String, OwnedValue> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), owned(value)))
        .collect()
}

#[test]
fn every_property_is_read_when_present() {
    let props = from_map(&map(vec![
        ("Title", "Dropbox".into()),
        ("Id", "dropbox-1".into()),
        ("Status", "Active".into()),
        (
            "Menu",
            Value::ObjectPath(ObjectPath::from_static_str_unchecked("/MenuBar")),
        ),
        ("ItemIsMenu", false.into()),
        ("IconName", "dropbox".into()),
    ]));
    assert_eq!(
        props,
        ItemProps {
            title: "Dropbox".into(),
            id: "dropbox-1".into(),
            passive: false,
            menu: Some("/MenuBar".into()),
            item_is_menu: false,
        }
    );
}

#[test]
fn passive_status_marks_the_item_passive_and_needs_attention_does_not() {
    let passive = from_map(&map(vec![("Status", "Passive".into())]));
    let attention = from_map(&map(vec![("Status", "NeedsAttention".into())]));
    assert!(passive.passive);
    assert!(!attention.passive);
}

#[test]
fn a_missing_menu_yields_no_menu_path() {
    assert_eq!(from_map(&map(vec![("Title", "A".into())])).menu, None);
}

#[test]
fn the_no_menu_sentinel_is_no_menu() {
    let props = from_map(&map(vec![(
        "Menu",
        Value::ObjectPath(ObjectPath::from_static_str_unchecked("/NO_DBUSMENU")),
    )]));
    assert_eq!(props.menu, None);
}

#[test]
fn an_absent_tooltip_and_every_other_optional_property_is_not_an_error() {
    let props = from_map(&map(Vec::new()));
    assert_eq!(props.title, "");
    assert_eq!(props.id, "");
    assert!(!props.passive);
    assert!(!props.item_is_menu);
}

#[test]
fn item_is_menu_is_read_as_a_flag() {
    assert!(from_map(&map(vec![("ItemIsMenu", true.into())])).item_is_menu);
}
