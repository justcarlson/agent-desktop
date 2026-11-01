use super::refusal;
use super::world::World;
use crate::system::x11::control::ControlRead;
use crate::system::x11::window::toplevel::Toplevel;
use crate::system::x11::window::window_kind::WindowKind;

/// One read of what a process shows that a close request must not answer: its
/// top-level windows, and the titles of the dialogs no top-level window stands
/// for, drawn inside a window or a dialog frame no toplevel matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Standing {
    pub(super) windows: Vec<Toplevel>,
    pub(super) nested: Vec<String>,
}

impl Standing {
    pub(super) fn of(world: &impl World, windows: Vec<Toplevel>) -> Self {
        let dialogs = world.dialogs();
        let mut windows = windows;
        let mut nested = dialogs.nested;
        nested.extend(mark_framed(&mut windows, dialogs.framed));
        Self { windows, nested }
    }

    pub(super) fn read(world: &impl World) -> Option<Self> {
        match world.toplevels() {
            ControlRead::Ready(windows) => Some(Self::of(world, windows)),
            _ => None,
        }
    }

    pub(super) fn has_dialog(&self) -> bool {
        !self.nested.is_empty() || self.windows.iter().any(refusal::is_dialog)
    }
}

/// Marks as a dialog the toplevel each dialog frame stands for, matched by
/// title among the toplevels no frame has claimed, a dialog the compositor
/// already marked first. Returns the titles no toplevel matches, which still
/// count as open dialogs.
fn mark_framed(windows: &mut [Toplevel], framed: Vec<String>) -> Vec<String> {
    let mut claimed = vec![false; windows.len()];
    let mut unmatched = Vec::new();
    for title in framed {
        let open = |index: &usize| !claimed[*index] && windows[*index].title == title;
        let found = (0..windows.len())
            .filter(open)
            .find(|index| refusal::is_dialog(&windows[*index]))
            .or_else(|| (0..windows.len()).find(open));
        match found {
            Some(index) => {
                claimed[index] = true;
                windows[index].kind = WindowKind::Dialog;
            }
            None => unmatched.push(title),
        }
    }
    unmatched
}

#[cfg(test)]
#[path = "standing_tests.rs"]
mod tests;
