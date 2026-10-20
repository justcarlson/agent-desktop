use super::*;
use crate::tree::test_support::words;

fn component() -> Field<Vec<String>> {
    Field::Present(vec![COMPONENT_INTERFACE.to_owned()])
}

fn no_states() -> Field<Vec<u32>> {
    Field::Present(words(&[]))
}

fn modal() -> Field<Vec<u32>> {
    Field::Present(words(&[16]))
}

#[test]
fn frame_dialog_and_window_roles_with_a_component_are_windows() {
    for role in [23, 16, 69] {
        assert!(
            is_window_shape(&Field::Present(role), &component(), &no_states()),
            "{role}"
        );
    }
}

#[test]
fn a_modal_panel_is_a_window_and_a_plain_panel_is_not() {
    let panel = Field::Present(39);
    assert!(is_window_shape(&panel, &component(), &modal()));
    assert!(!is_window_shape(&panel, &component(), &no_states()));
    assert!(!is_window_shape(
        &panel,
        &component(),
        &Field::Failed("x".into())
    ));
    assert!(!is_window_shape(&panel, &Field::Present(vec![]), &modal()));
}

#[test]
fn other_roles_and_frames_without_a_component_are_not() {
    let none = no_states();
    assert!(!is_window_shape(
        &Field::Present(75),
        &component(),
        &modal()
    ));
    assert!(!is_window_shape(
        &Field::Present(23),
        &Field::Present(vec![]),
        &none
    ));
    assert!(!is_window_shape(
        &Field::Present(23),
        &Field::Failed("x".into()),
        &none
    ));
    assert!(!is_window_shape(
        &Field::Failed("x".into()),
        &component(),
        &none
    ));
}

#[test]
fn the_shell_and_the_frame_decorator_are_helpers_by_either_name() {
    assert!(is_helper_app("mutter-x11-frames", None));
    assert!(is_helper_app("x", Some("mutter-x11-frames")));
    assert!(is_helper_app("gnome-shell", Some("gnome-shell")));
    assert!(!is_helper_app("org.gnome.Nautilus", Some("nautilus")));
}

#[test]
fn the_recorded_chrome_case_keeps_two_windows_and_drops_four_bubbles() {
    let frames = [
        (Some("BrowserRootView"), "about:blank - Google Chrome"),
        (Some("BrowserRootView"), "Second - Google Chrome"),
        (Some("RootView"), ""),
        (Some("RootView"), ""),
        (Some("RootView"), "  "),
        (Some("RootView"), ""),
    ];
    let kept = frames
        .iter()
        .filter(|(class, title)| !is_bubble(*class, title))
        .count();
    assert_eq!(kept, 2);
}

#[test]
fn a_titled_root_view_and_a_frame_with_no_class_are_windows() {
    assert!(!is_bubble(Some("RootView"), "Titled"));
    assert!(!is_bubble(None, ""));
}

#[test]
fn state_bits_become_visible_minimized_and_active() {
    let shown = frame_state(&Field::Present(words(&[25, 1])));
    assert_eq!(
        shown,
        FrameState {
            visible: Some(true),
            minimized: Some(false),
            active: Some(true),
            modal: Some(false)
        }
    );
    let hidden = frame_state(&Field::Present(words(&[15])));
    assert_eq!(hidden.visible, Some(false));
    assert_eq!(hidden.minimized, Some(true));
}

#[test]
fn an_unreadable_state_set_is_unknown_not_false() {
    let state = frame_state(&Field::Failed("x".into()));
    assert_eq!(
        state,
        FrameState {
            visible: None,
            minimized: None,
            active: None,
            modal: None
        }
    );
}

#[test]
fn a_showing_visible_filler_is_a_window_and_a_hidden_one_is_not() {
    let filler = Field::Present(20);
    let shown = Field::Present(words(&[25, 30]));
    assert!(is_window_shape(&filler, &component(), &shown));
    assert!(!is_window_shape(&filler, &component(), &no_states()));
    assert!(!is_window_shape(
        &filler,
        &component(),
        &Field::Present(words(&[25]))
    ));
    assert!(!is_window_shape(&filler, &Field::Present(vec![]), &shown));
}
