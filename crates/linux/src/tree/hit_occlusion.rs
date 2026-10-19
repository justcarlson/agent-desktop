use super::Extents;
use super::hit_guard::admits;
use crate::system::x11::desktop_placement::DesktopPlacement;
use crate::system::x11::stacked_window::StackedWindow;
use crate::system::x11::stacking_snapshot::StackingSnapshot;
use crate::system::x11::window_kind::WindowKind;

/// What the stacking order says about a point of the target's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occlusion {
    /// A managed window above the target, whose frame contains the point.
    CoveredBy {
        window: u32,
        frame: Extents,
    },
    Clear,
    /// A mapped override-redirect window above the target contains the point:
    /// real, but nothing proves it takes the event.
    OverrideRedirectOver,
    Unknown,
}

/// The border drawn around a client, so the rectangle tested is the frame and
/// not the client: a title bar covers a point the client rectangle does not.
fn frame_of(window: &StackedWindow) -> Option<Extents> {
    let grow = |edge: u32| i32::try_from(edge).ok();
    let (left, right, top, bottom) = (
        grow(window.extents.left)?,
        grow(window.extents.right)?,
        grow(window.extents.top)?,
        grow(window.extents.bottom)?,
    );
    Some(Extents {
        x: window.client.x.checked_sub(left)?,
        y: window.client.y.checked_sub(top)?,
        width: window.client.width.checked_add(left)?.checked_add(right)?,
        height: window.client.height.checked_add(top)?.checked_add(bottom)?,
    })
}

fn on_screen_now(window: &StackedWindow, current: Option<u32>) -> Option<bool> {
    match (window.desktop, current) {
        (DesktopPlacement::Sticky, _) => Some(true),
        (DesktopPlacement::On(desktop), Some(current)) => Some(desktop == current),
        (DesktopPlacement::On(_) | DesktopPlacement::Unknown, _) => None,
    }
}

/// Whether anything above the target in `snapshot` covers `point`, a point in
/// root coordinates. Windows that are hidden, unmapped, on another desktop or
/// of the desktop type are not above anything the user sees. A window whose
/// desktop cannot be read and that contains the point is `Unknown`.
pub(crate) fn occlusion(snapshot: &StackingSnapshot, target: u32, point: (i32, i32)) -> Occlusion {
    let Some(position) = snapshot.stack.iter().position(|window| window.id == target) else {
        return Occlusion::Unknown;
    };
    let own = &snapshot.stack[position];
    if own.flags.hidden
        || !own.flags.mapped
        || on_screen_now(own, snapshot.current_desktop) != Some(true)
    {
        return Occlusion::Unknown;
    }
    let mut unreadable = false;
    for window in snapshot.stack[position + 1..].iter().rev() {
        if window.flags.hidden || !window.flags.mapped || window.kind == WindowKind::Desktop {
            continue;
        }
        let Some(frame) = frame_of(window) else {
            continue;
        };
        if !admits(frame, point) {
            continue;
        }
        match on_screen_now(window, snapshot.current_desktop) {
            Some(true) => {
                return Occlusion::CoveredBy {
                    window: window.id,
                    frame,
                };
            }
            Some(false) => {}
            None => unreadable = true,
        }
    }
    if unreadable {
        return Occlusion::Unknown;
    }
    if snapshot
        .override_redirect_above
        .iter()
        .any(|rect| admits(*rect, point))
    {
        Occlusion::OverrideRedirectOver
    } else {
        Occlusion::Clear
    }
}

#[cfg(test)]
#[path = "hit_occlusion_tests.rs"]
mod tests;
