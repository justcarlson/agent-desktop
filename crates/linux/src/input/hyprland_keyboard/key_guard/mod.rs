//! The key guard of a keyboard command on a bound Hyprland session. Hyprland
//! keeps a destroyed virtual keyboard's pressed keys held, and a killed
//! command runs no cleanup, so the CLI starts this executable again as a
//! guard before its first key-down. The guard runs in a session of its own,
//! waits on a pidfd for the command, and once the command is gone releases
//! the keys the command's own journal record still holds through a new
//! virtual keyboard. A command that ended cleanly left no record, and the
//! guard exits without connecting. A library host starts no guard; the
//! journal stays the fallback either way.

pub(crate) mod guard_entry;
pub(crate) mod guard_payload;
pub(crate) mod guard_record;
pub(crate) mod guard_release;
pub(crate) mod guard_spawn;
