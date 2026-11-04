use agent_desktop_core::Rect;

use crate::tree::Extents;

/// The part of an element's window rectangle that its scroll container
/// shows, or `None` when the element has no area or lies wholly outside the
/// container.
pub(crate) fn aim_rect(node: Option<Extents>, container: Option<Extents>) -> Option<Rect> {
    let node = Rect::from(node?);
    let rect = match container {
        Some(container) => intersect(&node, &Rect::from(container))?,
        None => node,
    };
    (rect.width > 0.0 && rect.height > 0.0).then_some(rect)
}

fn intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > left && bottom > top).then_some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

#[cfg(test)]
#[path = "aim_rect_tests.rs"]
mod tests;
