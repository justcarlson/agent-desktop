use agent_desktop_core::{WindowInfo, WindowState};

use super::focused_index;
use crate::system::listed_window::ListedWindow;
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
        frame: FrameFacts {
            pid: 7,
            title: title.to_owned(),
            extents: None,
            global_position: true,
        },
        sheet: false,
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

#[test]
fn a_frame_that_reports_active_and_matches_the_display_is_focused() {
    let listed = [window("main", true, false), window("dialog", false, false)];
    assert_eq!(focused_index(&listed, &state(1)), Some(0));
}

#[test]
fn a_modal_frame_that_never_reports_active_is_focused_when_the_display_names_it() {
    let listed = [window("main", false, false), window("dialog", false, true)];
    assert_eq!(focused_index(&listed, &state(2)), Some(1));
}

#[test]
fn a_modal_frame_the_display_does_not_name_is_not_focused() {
    let listed = [window("main", false, false), window("dialog", false, true)];
    assert_eq!(focused_index(&listed, &state(1)), None);
}

#[test]
fn a_frame_that_is_not_modal_and_not_active_is_never_focused_by_the_display_alone() {
    let listed = [window("main", false, false), window("dialog", false, false)];
    assert_eq!(focused_index(&listed, &state(2)), None);
}
