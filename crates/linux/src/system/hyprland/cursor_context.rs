use super::bind::hyprland_binding::HyprlandBinding;

/// What the cursor hider factory knows about the command: the bound
/// compositor whose overlays it hides.
pub(crate) struct CursorContext<'a> {
    pub(crate) binding: &'a HyprlandBinding,
}
