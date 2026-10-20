use agent_desktop_core::{WindowInfo, WindowState};

use super::surfaces_of;
use crate::system::listed_window::ListedWindow;
use crate::system::x11::FrameFacts;

fn window(id: &str, title: &str, sheet: bool) -> ListedWindow {
    ListedWindow {
        app: 0,
        info: WindowInfo {
            id: id.to_owned(),
            title: title.to_owned(),
            app: "a".to_owned(),
            pid: 7.into(),
            process_instance: Some("i".to_owned()),
            bounds: None,
            state: WindowState::default(),
        },
        active: false,
        modal: false,
        frame: FrameFacts {
            pid: 7,
            title: title.to_owned(),
            extents: None,
            global_position: true,
        },
        sheet,
    }
}

#[test]
fn every_window_lists_a_window_surface_with_its_id_and_title() {
    let surfaces = surfaces_of(&[window("w-1", "Doc", false)]);
    let [surface] = surfaces.as_slice() else {
        panic!("one surface")
    };
    assert_eq!(surface.id, "w-1");
    assert_eq!(surface.kind, "window");
    assert_eq!(surface.title.as_deref(), Some("Doc"));
}

#[test]
fn a_modal_dialog_also_lists_a_sheet_surface_on_the_same_id() {
    let surfaces = surfaces_of(&[window("w-1", "Save", true)]);
    let kinds: Vec<&str> = surfaces
        .iter()
        .map(|surface| surface.kind.as_str())
        .collect();
    assert_eq!(kinds, ["window", "sheet"]);
    assert!(surfaces.iter().all(|surface| surface.id == "w-1"));
}

#[test]
fn an_untitled_window_lists_no_title() {
    let surfaces = surfaces_of(&[window("w-1", "", false)]);
    assert_eq!(surfaces[0].title, None);
}
