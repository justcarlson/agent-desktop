use agent_desktop_core::{AdapterError, Deadline, ErrorCode, ObservationRoot, SnapshotSurface};

use super::tests::{observe_plain, request, rooted_walk, window};
use crate::system::window_id;
use crate::tree::test_support::{read_of, words};
use crate::tree::walker::walk_levels;
use crate::tree::wrapper::classify;
use crate::tree::{AtspiElement, Expansion, Field, ObservedRead, Walk, WalkLimits};

fn window_holding_a_dialog(
    showing: bool,
) -> impl Fn(&WalkLimits) -> Result<Walk<ObservedRead>, AdapterError> {
    move |limits| {
        walk_levels(
            0u32,
            limits,
            Deadline::after(5_000).expect("deadline"),
            |level| {
                Ok(level
                    .iter()
                    .map(|key| {
                        if *key == 0 {
                            let mut frame = read_of(23, "w", &[]);
                            frame.node.traits.states = Field::Present(words(&[25]));
                            Expansion::read(frame, vec![1u32])
                        } else {
                            let mut dialog = read_of(16, "", &[]);
                            dialog.node.element = AtspiElement::new(":1.7", "/d", 77);
                            let bits: &[u32] = if showing { &[25] } else { &[] };
                            dialog.node.traits.states = Field::Present(words(bits));
                            Expansion::read(dialog, Vec::new())
                        }
                    })
                    .collect())
            },
            classify,
        )
    }
}

#[test]
fn a_sheet_surface_roots_at_a_dialog_drawn_inside_the_window() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let mut sheet = request();
    sheet.surface = SnapshotSurface::Sheet;
    let holding = window_holding_a_dialog(true);
    let dialog_walk = rooted_walk(16, &[25]);
    let mut rooted_at = Vec::new();
    let tree = observe_plain(
        ObservationRoot::Window(&window),
        &sheet,
        |element, limits, _| {
            rooted_at.push(element.path().to_owned());
            if element.path() == "/a" {
                holding(limits)
            } else {
                dialog_walk(limits)
            }
        },
    )
    .expect("the nested dialog observes as the sheet");
    assert!(tree.is_complete());
    assert_eq!(rooted_at, ["/a", "/d"]);
}

#[test]
fn a_dialog_inside_the_window_that_does_not_show_is_no_sheet() {
    let id = window_id::encode(":1.7", "/a").expect("encodes");
    let window = window(id);
    let mut sheet = request();
    sheet.surface = SnapshotSurface::Sheet;
    let holding = window_holding_a_dialog(false);
    let error = observe_plain(ObservationRoot::Window(&window), &sheet, |_, limits, _| {
        holding(limits)
    })
    .expect_err("a hidden dialog is not a sheet");
    assert_eq!(error.code, ErrorCode::ElementNotFound);
}
