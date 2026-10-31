//! Capture over the compositor's own protocols on a connection the caller
//! has already checked: one output through `zwlr_screencopy_v1`, into shared
//! memory this process owns, and only from a frame the compositor presented
//! after the request.

pub(crate) mod buffer_layout;
pub(crate) mod capture_dispatch;
pub(crate) mod capture_fault;
pub(crate) mod capture_globals;
pub(crate) mod capture_state;
pub(crate) mod composition;
pub(crate) mod frame_failure;
pub(crate) mod frame_freshness;
pub(crate) mod frame_outcome;
pub(crate) mod frame_record;
pub(crate) mod frame_redraw;
pub(crate) mod frame_settle;
pub(crate) mod output_capture_globals;
pub(crate) mod output_frame;
pub(crate) mod raw_frame;
pub(crate) mod screencopy_record;
pub(crate) mod session_record;
pub(crate) mod shm_buffer;
pub(crate) mod shm_layout;
pub(crate) mod toplevel_capture_globals;
pub(crate) mod toplevel_frame;
