//! Keyboard delivery on a bound Hyprland session through
//! `zwp_virtual_keyboard_v1` on the bound compositor's own Wayland socket.
//! The virtual keyboard types with the seat owner's keymap extended by the
//! keysyms a delivery needs, in the layout the owner has active, and is
//! destroyed when the delivery ends so the compositor returns to the
//! physical keyboard's keymap. Held keys are journaled; when the CLI is
//! killed, its key guard releases what the journal holds, and the next
//! delivery on the instance releases anything left.

pub(crate) mod active_layout;
mod control_text;
#[cfg(test)]
pub(crate) mod fake_wire;
pub(crate) mod held_key_journal;
pub(crate) mod held_keys;
pub(crate) mod key_guard;
pub(crate) mod keyboard_cancel;
pub(crate) mod keyboard_state;
pub(crate) mod keyboard_wire;
pub(crate) mod keymap_fd;
pub(crate) mod record_claim;
pub(crate) mod stale_key_release;
pub(crate) mod virtual_keyboard_delivery;
pub(crate) mod virtual_keyboard_sink;
pub(crate) mod wayland_wire;
