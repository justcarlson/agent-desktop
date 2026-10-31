use super::*;

#[test]
fn an_id_names_the_compositor_process_and_its_toplevel_identifier() {
    let id = encode(1234, 98765, "18000035").expect("an id");

    assert_eq!(id, "w-hypr-1234-98765-18000035");
    assert!(is_well_formed(&id));
    assert!(is_compositor_id(&id));
}

#[test]
fn a_restarted_compositor_issues_a_different_id_for_the_same_identifier() {
    assert_ne!(encode(1234, 98765, "1a"), encode(1234, 98766, "1a"));
    assert_ne!(encode(1234, 98765, "1a"), encode(1235, 98765, "1a"));
}

#[test]
fn an_identifier_that_is_not_plain_alphanumeric_gets_no_id() {
    assert_eq!(encode(1, 2, ""), None);
    assert_eq!(encode(1, 2, "18-00"), None);
    assert_eq!(encode(1, 2, "a b"), None);
    assert_eq!(encode(1, 2, &"a".repeat(65)), None);
}

#[test]
fn only_the_exact_encoded_form_is_well_formed() {
    for id in [
        "w-1.42-org-a11y-atspi-accessible-1",
        "w-hypr-1234-98765",
        "w-hypr-01234-98765-1a",
        "w-hypr-1234-98765-1a-2",
        "w-hypr--98765-1a",
        "hypr-1234-98765-1a",
        "w-hypr-1234-x-1a",
    ] {
        assert!(!is_well_formed(id), "{id}");
    }
}

#[test]
fn an_accessibility_window_id_never_has_the_compositor_prefix() {
    assert!(!is_compositor_id("w-1.42-org-a11y-atspi-accessible-1"));
    assert!(is_compositor_id("w-hypr-garbage"));
}
