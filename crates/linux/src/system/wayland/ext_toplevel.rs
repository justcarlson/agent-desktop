use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

/// One `ext_foreign_toplevel_handle_v1` and the identifier it reported. A
/// handle the compositor closed has an empty identifier, so it names no
/// window.
#[derive(Debug, Clone)]
pub(crate) struct ExtToplevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) identifier: String,
}
