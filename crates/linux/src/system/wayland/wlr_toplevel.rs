use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;

/// One `zwlr_foreign_toplevel_handle_v1` and what it last reported: its
/// title, whether it is the activated toplevel, the toplevel it is a child
/// of, and whether the compositor closed it.
#[derive(Debug, Clone)]
pub(crate) struct WlrToplevel {
    pub(crate) handle: ZwlrForeignToplevelHandleV1,
    pub(crate) title: String,
    pub(crate) activated: bool,
    pub(crate) parent: Option<ZwlrForeignToplevelHandleV1>,
    pub(crate) closed: bool,
}
