use super::*;

fn placed(node: Extents, frame: Extents) -> Placement {
    Placement {
        node: Some(node),
        frame: Some(frame),
        ..Placement::default()
    }
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn the_display_server_is_skipped_only_when_nothing_more_is_wanted_from_it() {
    assert!(skips_display_server(true, false, false));
    assert!(skips_display_server(false, true, true));
    assert!(!skips_display_server(false, true, false));
    assert!(!skips_display_server(false, false, true));
    assert!(!skips_display_server(false, false, false));
}

#[test]
fn a_node_wholly_outside_its_window_is_decided_without_the_display_server() {
    let frame = rect(0, 0, 420, 420);
    assert!(wholly_outside(&placed(rect(2, 5526, 420, 22), frame)));
    assert!(!wholly_outside(&placed(rect(2, 20, 420, 22), frame)));
    assert!(!wholly_outside(&Placement::default()));
}
