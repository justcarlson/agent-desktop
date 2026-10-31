//! `focus-window` on Hyprland: bind the window to one toplevel, prove focus
//! from the compositor and, for an accessibility frame, from the
//! application, and activate through the foreign toplevel protocol only
//! when the command is headed.

#[cfg(test)]
pub(crate) mod fake_snapshot;
pub(crate) mod focus_entry;
pub(crate) mod focus_refusal;
pub(crate) mod hypr_focus;
pub(crate) mod hypr_world;
#[cfg(test)]
pub(crate) mod hypr_world_fake;
pub(crate) mod live_hypr_world;
pub(crate) mod proof_read;
