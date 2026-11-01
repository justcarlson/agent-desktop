//! The Hyprland cursor renderer: one long-lived `agent-desktop` process per
//! session that draws the agent cursor on a `zwlr_layer_shell_v1` overlay
//! surface with an empty input region and no keyboard interactivity. The
//! caller reaches it over a per-session Unix socket under
//! `$XDG_RUNTIME_DIR/agent-desktop/cursor/`, starts it lazily under a start
//! lock, and tears it down through a pidfd.

pub(crate) mod agent_cursor;
pub(crate) mod arrow_bitmap;
pub(crate) mod cursor_animation;
pub(crate) mod cursor_surface;
pub(crate) mod exe_identity;
pub(crate) mod frame_mark;
pub(crate) mod holder_file;
pub(crate) mod holder_view;
pub(crate) mod hypr_cursor;
pub(crate) mod layer_client;
pub(crate) mod layer_globals;
pub(crate) mod layer_link;
pub(crate) mod layer_state;
pub(crate) mod live_launcher;
pub(crate) mod placement;
pub(crate) mod record_class;
pub(crate) mod record_read;
pub(crate) mod renderer_entry;
pub(crate) mod renderer_launcher;
pub(crate) mod renderer_owner;
pub(crate) mod renderer_paths;
pub(crate) mod renderer_pidfd;
pub(crate) mod renderer_process;
pub(crate) mod renderer_record;
pub(crate) mod renderer_reply;
pub(crate) mod renderer_request;
pub(crate) mod renderer_spawn;
pub(crate) mod renderer_start;
pub(crate) mod renderer_stop;
pub(crate) mod renderer_target;
pub(crate) mod renderer_wire;
pub(crate) mod session_stop;
pub(crate) mod sha256;
pub(crate) mod shm_canvas;
pub(crate) mod stop_outcome;
pub(crate) mod stop_timings;

#[cfg(test)]
pub(crate) mod fake_layer_compositor;
#[cfg(test)]
pub(crate) mod registry_fixture;
