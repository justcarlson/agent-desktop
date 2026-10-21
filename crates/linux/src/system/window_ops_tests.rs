use agent_desktop_core::{WindowInfo, WindowState};

use crate::system::listed_window::ListedWindow;
use crate::system::signal_inventory::pass_over_with_menu;
use crate::system::x11::{FrameExtents, FrameFacts, X11Client, X11State};

fn window(title: &str, active: bool, modal: bool) -> ListedWindow {
    ListedWindow {
        app: 0,
        info: WindowInfo {
            id: format!("w-1.1-{title}"),
            title: title.to_owned(),
            app: "a".to_owned(),
            pid: 7.into(),
            process_instance: Some("i".to_owned()),
            bounds: None,
            state: WindowState::default(),
        },
        active,
        modal,
        dialog: false,
        frame: FrameFacts {
            pid: 7,
            title: title.to_owned(),
            extents: None,
            global_position: true,
        },
        sheet: false,
        unshown: false,
    }
}

fn client(id: u32, title: &str) -> X11Client {
    X11Client {
        shadow: (0, 0, 0, 0),
        id,
        pid: Some(7),
        title: title.to_owned(),
        extents: None::<FrameExtents>,
        bounds: None,
        hidden: false,
    }
}

fn state(active: u32) -> X11State {
    X11State::Ready {
        active: Some(active),
        clients: vec![client(1, "main"), client(2, "dialog")],
    }
}

fn focused(
    listed: Vec<ListedWindow>,
    state: &X11State,
    input_focus_in: impl Fn(u32) -> bool,
) -> Option<usize> {
    pass_over_with_menu(
        listed,
        Vec::new(),
        Vec::new(),
        state,
        |_| Ok(false),
        input_focus_in,
    )
    .focused
}

#[test]
fn a_modal_frame_that_never_reports_active_is_focused_when_the_display_and_input_focus_name_it() {
    let listed = vec![window("main", false, false), window("dialog", false, true)];
    assert_eq!(focused(listed, &state(2), |_| true), Some(1));
}

#[test]
fn a_dialog_frame_the_display_does_not_name_is_not_focused() {
    let mut dialog = window("dialog", false, false);
    dialog.dialog = true;
    let listed = vec![window("main", false, false), dialog];
    assert_eq!(focused(listed, &state(1), |_| false), None);
}

#[test]
fn a_modal_frame_the_display_does_not_name_is_not_focused() {
    let listed = vec![window("main", false, false), window("dialog", false, true)];
    assert_eq!(focused(listed, &state(1), |_| false), None);
}

#[test]
fn a_frame_that_is_not_modal_and_not_active_is_never_focused_by_the_display_alone() {
    let listed = vec![window("main", false, false), window("dialog", false, false)];
    assert_eq!(focused(listed, &state(2), |_| true), None);
}

#[test]
fn an_inactive_dialog_needs_positive_input_focus_evidence() {
    for modal in [false, true] {
        let mut dialog = window("dialog", false, modal);
        dialog.dialog = true;
        let listed = vec![window("main", false, false), dialog];
        assert_eq!(focused(listed.clone(), &state(2), |_| false), None);
        assert_eq!(focused(listed.clone(), &state(2), |id| id == 2), Some(1));
        assert_eq!(focused(listed, &state(2), |id| id == 1), None);
    }
}
