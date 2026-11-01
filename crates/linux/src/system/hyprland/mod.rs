//! The Hyprland instance this process is bound to, and what only that
//! instance can say: its clients over IPC and the Hyprland address of each
//! foreign toplevel. Nothing here reads Hyprland IPC or binds a
//! Hyprland-only protocol before `bind` has paired one instance's IPC socket
//! with its Wayland socket.

pub(crate) mod bind;
pub(crate) mod capture;
pub(crate) mod clients;
pub(crate) mod close;
pub(crate) mod cursor_context;
pub(crate) mod cursor_hider;
pub(crate) mod cursor_renderer;
pub(crate) mod focus;
pub(crate) mod identity;
pub(crate) mod ipc;
pub(crate) mod no_cursor_hider;
pub(crate) mod placement;
pub(crate) mod protocol;
pub(crate) mod receiver;
pub(crate) mod toplevel;
pub(crate) mod window_op;

/// The hider every headed Hyprland command and every display capture use.
/// No overlay is drawn yet, so it hides nothing.
pub(crate) fn cursor_hider(
    context: &cursor_context::CursorContext<'_>,
) -> Box<dyn cursor_hider::CursorHider> {
    let _ = context.binding;
    Box::new(no_cursor_hider::NoCursorHider)
}
