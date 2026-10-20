use super::*;
use crate::system::x11::FrameExtents;
use crate::system::x11::window_flags::WindowFlags;

const TARGET: u32 = 10;
const POINT: (i32, i32) = (310, 154);

fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

fn window(id: u32, client: Extents) -> StackedWindow {
    StackedWindow {
        shadow: (0, 0),
        id,
        flags: WindowFlags {
            hidden: false,
            above: false,
            mapped: true,
        },
        desktop: DesktopPlacement::On(0),
        kind: WindowKind::Normal,
        client,
        extents: FrameExtents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
    }
}

fn target() -> StackedWindow {
    window(TARGET, rect(100, 100, 400, 300))
}

fn snapshot(stack: Vec<StackedWindow>) -> StackingSnapshot {
    StackingSnapshot {
        current_desktop: Some(0),
        stack,
        override_redirect_above: Vec::new(),
        override_redirect_receiving: Vec::new(),
        override_redirect_names: Vec::new(),
    }
}

fn cover() -> StackedWindow {
    window(20, rect(200, 100, 400, 300))
}

#[test]
fn a_window_above_the_target_containing_the_point_covers_it() {
    let read = snapshot(vec![target(), cover()]);
    assert_eq!(
        occlusion(&read, TARGET, POINT),
        Occlusion::CoveredBy {
            window: 20,
            frame: rect(200, 100, 400, 300)
        }
    );
}

#[test]
fn the_same_window_below_the_target_does_not() {
    let read = snapshot(vec![cover(), target()]);
    assert_eq!(occlusion(&read, TARGET, POINT), Occlusion::Clear);
}

#[test]
fn a_hidden_window_above_does_not_cover() {
    let mut hidden = cover();
    hidden.flags.hidden = true;
    let read = snapshot(vec![target(), hidden]);
    assert_eq!(occlusion(&read, TARGET, POINT), Occlusion::Clear);
}

#[test]
fn an_unmapped_window_above_does_not_cover() {
    let mut unmapped = cover();
    unmapped.flags.mapped = false;
    let read = snapshot(vec![target(), unmapped]);
    assert_eq!(occlusion(&read, TARGET, POINT), Occlusion::Clear);
}

#[test]
fn a_window_on_another_desktop_does_not_cover_and_a_sticky_one_does() {
    let mut elsewhere = cover();
    elsewhere.desktop = DesktopPlacement::On(1);
    assert_eq!(
        occlusion(&snapshot(vec![target(), elsewhere]), TARGET, POINT),
        Occlusion::Clear
    );
    let mut sticky = cover();
    sticky.desktop = DesktopPlacement::Sticky;
    assert!(matches!(
        occlusion(&snapshot(vec![target(), sticky]), TARGET, POINT),
        Occlusion::CoveredBy { window: 20, .. }
    ));
}

#[test]
fn a_window_whose_desktop_cannot_be_read_and_contains_the_point_is_unknown() {
    let mut unread = cover();
    unread.desktop = DesktopPlacement::Unknown;
    assert_eq!(
        occlusion(&snapshot(vec![target(), unread.clone()]), TARGET, POINT),
        Occlusion::Unknown
    );
    unread.client = rect(900, 900, 10, 10);
    assert_eq!(
        occlusion(&snapshot(vec![target(), unread]), TARGET, POINT),
        Occlusion::Clear
    );
}

#[test]
fn a_desktop_type_window_does_not_cover() {
    let mut desktop = cover();
    desktop.kind = WindowKind::Desktop;
    let read = snapshot(vec![target(), desktop]);
    assert_eq!(occlusion(&read, TARGET, POINT), Occlusion::Clear);
}

#[test]
fn a_window_whose_frame_reaches_the_point_only_through_its_title_bar_covers() {
    let mut titled = window(20, rect(200, 160, 400, 240));
    titled.extents = FrameExtents {
        left: 1,
        right: 1,
        top: 30,
        bottom: 1,
    };
    let read = snapshot(vec![target(), titled]);
    assert!(matches!(
        occlusion(&read, TARGET, (310, 140)),
        Occlusion::CoveredBy { window: 20, .. }
    ));
}

#[test]
fn the_client_rectangle_alone_would_miss_the_title_bar() {
    let titled = window(20, rect(200, 160, 400, 240));
    let read = snapshot(vec![target(), titled]);
    assert_eq!(occlusion(&read, TARGET, (310, 140)), Occlusion::Clear);
}

#[test]
fn a_window_above_that_misses_the_point_does_not_cover() {
    let read = snapshot(vec![target(), window(20, rect(600, 100, 100, 100))]);
    assert_eq!(occlusion(&read, TARGET, POINT), Occlusion::Clear);
}

#[test]
fn the_topmost_covering_window_is_named() {
    let read = snapshot(vec![
        target(),
        cover(),
        window(30, rect(250, 100, 100, 100)),
    ]);
    assert!(matches!(
        occlusion(&read, TARGET, (300, 150)),
        Occlusion::CoveredBy { window: 30, .. }
    ));
}

#[test]
fn an_override_redirect_window_over_the_point_is_reported_as_such() {
    let mut read = snapshot(vec![target()]);
    read.override_redirect_above = vec![rect(300, 150, 50, 50)];
    assert_eq!(
        occlusion(&read, TARGET, POINT),
        Occlusion::OverrideRedirectOver
    );
}

#[test]
fn an_override_redirect_window_whose_input_region_holds_the_point_covers_it() {
    let mut read = snapshot(vec![target()]);
    read.override_redirect_above = vec![rect(300, 150, 50, 50)];
    read.override_redirect_receiving = vec![(77, rect(300, 150, 50, 50))];
    assert_eq!(
        occlusion(&read, TARGET, POINT),
        Occlusion::CoveredBy {
            window: 77,
            frame: rect(300, 150, 50, 50)
        }
    );
}

#[test]
fn an_override_redirect_window_whose_input_region_misses_the_point_does_not_cover_it() {
    let mut read = snapshot(vec![target()]);
    read.override_redirect_above = vec![rect(300, 150, 50, 50)];
    read.override_redirect_receiving = vec![(77, rect(300, 150, 10, 10))];
    assert_eq!(
        occlusion(&read, TARGET, POINT),
        Occlusion::OverrideRedirectOver
    );
}

#[test]
fn a_managed_cover_outranks_an_override_redirect_window() {
    let mut read = snapshot(vec![target(), cover()]);
    read.override_redirect_above = vec![rect(300, 150, 50, 50)];
    assert!(matches!(
        occlusion(&read, TARGET, POINT),
        Occlusion::CoveredBy { .. }
    ));
}

#[test]
fn an_unmapped_hidden_or_off_desktop_target_is_unknown() {
    let mut unmapped = target();
    unmapped.flags.mapped = false;
    assert_eq!(
        occlusion(&snapshot(vec![unmapped]), TARGET, POINT),
        Occlusion::Unknown
    );
    let mut hidden = target();
    hidden.flags.hidden = true;
    assert_eq!(
        occlusion(&snapshot(vec![hidden]), TARGET, POINT),
        Occlusion::Unknown
    );
    let mut away = target();
    away.desktop = DesktopPlacement::On(2);
    assert_eq!(
        occlusion(&snapshot(vec![away]), TARGET, POINT),
        Occlusion::Unknown
    );
}

#[test]
fn a_target_missing_from_the_stack_is_unknown() {
    assert_eq!(
        occlusion(&snapshot(vec![cover()]), TARGET, POINT),
        Occlusion::Unknown
    );
}
