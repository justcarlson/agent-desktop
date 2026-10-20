use super::MenuContext;
use crate::tree::test_support::ref_entry;

#[test]
fn a_menu_ref_is_rejected_when_a_different_popup_reuses_the_item_path() {
    let captured = MenuContext {
        root: "/org/a11y/atspi/accessible/10".to_owned(),
        shape: 100,
        complete: true,
    };
    let replacement = MenuContext {
        root: "/org/a11y/atspi/accessible/20".to_owned(),
        shape: 200,
        complete: true,
    };
    let mut entry = ref_entry("menuitem");
    entry.identity.retained_object = Some(captured.encode("/org/a11y/atspi/accessible/10/0"));

    assert!(replacement.entry_for(&entry).is_none());
}

#[test]
fn a_truncated_popup_prefix_cannot_match_or_resolve_a_same_shape_ref() {
    let incomplete = MenuContext {
        root: "/org/a11y/atspi/accessible/menu".to_owned(),
        shape: 100,
        complete: false,
    };
    let mut entry = ref_entry("menuitem");
    entry.identity.retained_object = Some(incomplete.encode("/org/a11y/atspi/accessible/menu/0"));

    assert!(!incomplete.matches(&incomplete));
    assert!(incomplete.entry_for(&entry).is_none());
}

#[test]
fn the_captured_popup_context_restores_the_original_item_path_for_resolution() {
    let context = MenuContext {
        root: "/org/a11y/atspi/accessible/10".to_owned(),
        shape: 100,
        complete: true,
    };
    let mut entry = ref_entry("menuitem");
    entry.identity.retained_object = Some(context.encode("/org/a11y/atspi/accessible/10/0"));

    let scoped = context.entry_for(&entry).expect("same popup context");
    assert_eq!(
        scoped.identity.retained_object.as_deref(),
        Some("/org/a11y/atspi/accessible/10/0")
    );
}

#[test]
fn a_menu_context_round_trips_utf8_paths_and_item_paths() {
    let context = MenuContext {
        root: "/org/a11y/atspi/accessible/メニュー".to_owned(),
        shape: 123,
        complete: true,
    };
    let token = context.encode("/org/a11y/atspi/accessible/メニュー/0");

    let (decoded, object) = MenuContext::decode(&token).expect("valid context token");
    assert!(context.matches(&decoded));
    assert_eq!(object, "/org/a11y/atspi/accessible/メニュー/0");
}
