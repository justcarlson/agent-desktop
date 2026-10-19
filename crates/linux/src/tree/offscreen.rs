use super::Extents;

/// The widest a rectangle may sit outside a frame or a scroll container and
/// still not count as off it, in pixels. Toolkits draw a window a few pixels
/// beyond the rectangle they report (the widest in-frame overhang measured on
/// a showing node was 8 px on GTK 4 Files, L27-2, and 17 px of frame offset
/// was seen under another window manager, L6-6), so a node only that far out
/// proves nothing.
pub(crate) const EDGE_SLACK: i32 = 24;

/// Everything the offscreen rule reads about one node. Every rectangle is in
/// window coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OffscreenInput {
    pub(crate) node: Option<Extents>,
    pub(crate) frame: Option<Extents>,
    pub(crate) container: Option<Extents>,
    pub(crate) visible: bool,
    pub(crate) showing: bool,
    pub(crate) sentinel: bool,
    pub(crate) minimized: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Inside,
    Near,
    Outside,
}

fn usable(rect: Option<Extents>) -> Option<Extents> {
    rect.filter(|rect| {
        rect.width > 0 && rect.height > 0 && rect.x != i32::MIN && rect.y != i32::MIN
    })
}

fn axis_gap(start: i32, length: i32, other_start: i32, other_length: i32) -> i64 {
    let end = i64::from(start) + i64::from(length);
    let other_end = i64::from(other_start) + i64::from(other_length);
    (i64::from(start) - other_end).max(i64::from(other_start) - end)
}

fn region(node: Extents, around: Extents) -> Region {
    let across = axis_gap(node.x, node.width, around.x, around.width);
    let down = axis_gap(node.y, node.height, around.y, around.height);
    let gap = across.max(down);
    if gap < 0 {
        Region::Inside
    } else if gap > i64::from(EDGE_SLACK) {
        Region::Outside
    } else {
        Region::Near
    }
}

/// `Some(true)` only when the node is provably off screen, `Some(false)` when
/// it provably intersects what shows it, `None` otherwise.
///
/// Off screen is a minimized window, the GTK 3 shape of a row scrolled out of
/// its viewport (`VISIBLE` without `SHOWING` at the sentinel position, L6-9),
/// or a rectangle of positive area wholly outside the frame, or the scroll
/// container, by more than `EDGE_SLACK`. `SHOWING` being absent is never
/// enough on its own: on-screen Chromium and Gecko nodes read `VISIBLE`
/// without it (L3-27). An unknown minimized state keeps a node that is
/// otherwise inside from being called on screen.
pub(crate) fn offscreen(input: OffscreenInput) -> Option<bool> {
    if input.minimized == Some(true) {
        return Some(true);
    }
    if input.sentinel && input.visible && !input.showing {
        return Some(true);
    }
    let node = usable(input.node)?;
    let frame = usable(input.frame)?;
    let container = match input.container {
        Some(container) => Some(usable(Some(container))?),
        None => None,
    };
    let mut inside = true;
    for around in std::iter::once(frame).chain(container) {
        match region(node, around) {
            Region::Outside => return Some(true),
            Region::Near => inside = false,
            Region::Inside => {}
        }
    }
    (inside && input.minimized == Some(false)).then_some(false)
}

#[cfg(test)]
#[path = "offscreen_tests.rs"]
mod tests;
