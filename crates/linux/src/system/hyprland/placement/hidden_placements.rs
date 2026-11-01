//! The placements this command found hidden: a target window on no shown
//! workspace, or with its centre on no output. The cursor's `present` to a
//! point inside one is sent parked, so the cursor draws nothing over the
//! workspace that is shown. A placement read again as shown drops the
//! hidden ones under its centre. Placements are kept on the thread that
//! reads them, which is the thread that sends the command's `present`, and
//! only for the life of one command.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use agent_desktop_core::{Point, Rect};

use super::layout_placement::Placement;

const KEEP_FOR: Duration = Duration::from_secs(10);

thread_local! {
    static HIDDEN: RefCell<Vec<(Rect, Instant)>> = const { RefCell::new(Vec::new()) };
}

/// Keeps `placement` when it is hidden; a shown one drops every kept
/// placement under its centre.
pub(crate) fn keep(placement: &Placement) {
    HIDDEN.with_borrow_mut(|hidden| {
        hidden.retain(|(_, kept)| kept.elapsed() < KEEP_FOR);
        if placement.shown {
            let centre = centre(&placement.rect);
            hidden.retain(|(rect, _)| !inside(rect, &centre));
        } else if !hidden.iter().any(|(rect, _)| *rect == placement.rect) {
            hidden.push((placement.rect, Instant::now()));
        }
    });
}

/// Whether `point` lies in a hidden placement kept on this thread.
pub(crate) fn hides(point: &Point) -> bool {
    HIDDEN.with_borrow(|hidden| {
        hidden
            .iter()
            .any(|(rect, kept)| kept.elapsed() < KEEP_FOR && inside(rect, point))
    })
}

fn centre(rect: &Rect) -> Point {
    Point {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}

fn inside(rect: &Rect, point: &Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}
