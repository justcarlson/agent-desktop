use super::*;

#[test]
fn an_id_names_the_compositor_process_and_its_toplevel_identifier() {
    let id = encode(1234, 98765, "18000035").expect("an id");

    assert_eq!(id, "w-hypr-1234-98765-18000035");
    assert_eq!(
        decode(&id),
        Some(CompositorWindowId {
            compositor_pid: 1234,
            compositor_start: 98765,
            identifier: "18000035".into(),
        })
    );
}

#[test]
fn an_identifier_that_is_not_plain_alphanumeric_gets_no_id() {
    assert_eq!(encode(1, 2, ""), None);
    assert_eq!(encode(1, 2, "18-00"), None);
    assert_eq!(encode(1, 2, "a b"), None);
    assert_eq!(encode(1, 2, &"a".repeat(65)), None);
}

#[test]
fn only_the_exact_encoded_form_decodes() {
    for id in [
        "w-1.42-org-a11y-atspi-accessible-1",
        "w-hypr-1234-98765",
        "w-hypr-01234-98765-1a",
        "w-hypr-1234-98765-1a-2",
        "w-hypr--98765-1a",
        "hypr-1234-98765-1a",
        "w-hypr-1234-x-1a",
    ] {
        assert_eq!(decode(id), None, "{id}");
    }
}

#[test]
fn an_id_belongs_only_to_the_compositor_instance_that_issued_it() {
    let id = decode("w-hypr-1234-98765-1a").expect("decodes");

    assert!(id.issued_by(1234, 98765));
    assert!(!id.issued_by(1234, 98766));
    assert!(!id.issued_by(1235, 98765));
}
