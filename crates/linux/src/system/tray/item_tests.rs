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
    let props = from_map(
        &map(vec![
            ("Title", "Dropbox".into()),
            ("Status", "Active".into()),
            (
                "Menu",
                Value::ObjectPath(ObjectPath::from_static_str_unchecked("/MenuBar")),
            ),
            ("IconName", "dropbox".into()),
        ]),
        Vec::new(),
    );
    assert_eq!(
        props,
        ItemEvidence {
            title: "Dropbox".into(),
            passive: false,
            menu: Some("/MenuBar".into()),
            methods: Vec::new(),
        }
    );
}

#[test]
fn passive_status_marks_the_item_passive_and_needs_attention_does_not() {
    let passive = from_map(&map(vec![("Status", "Passive".into())]), Vec::new());
    let attention = from_map(&map(vec![("Status", "NeedsAttention".into())]), Vec::new());
    assert!(passive.passive);
    assert!(!attention.passive);
}

#[test]
fn a_missing_menu_yields_no_menu_path() {
    assert_eq!(
        from_map(&map(vec![("Title", "A".into())]), Vec::new()).menu,
        None
    );
}

#[test]
fn the_no_menu_sentinel_is_no_menu() {
    let props = from_map(
        &map(vec![(
            "Menu",
            Value::ObjectPath(ObjectPath::from_static_str_unchecked("/NO_DBUSMENU")),
        )]),
        Vec::new(),
    );
    assert_eq!(props.menu, None);
}

#[test]
fn an_absent_tooltip_and_every_other_optional_property_is_not_an_error() {
    let props = from_map(&map(Vec::new()), Vec::new());
    assert_eq!(props.title, "");
    assert!(!props.passive);
}
