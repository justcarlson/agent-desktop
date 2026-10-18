/// Which display server this session's reads may reach. `WAYLAND_DISPLAY`
/// decides before `DISPLAY` because an Xwayland client still lives on a
/// Wayland desktop whose windows X11 calls cannot enumerate; a headless
/// session reaches neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DisplayServer {
    Wayland,
    X11(String),
    None,
}

/// The session's display server from its environment, deciding Wayland
/// before X11 for the Xwayland case.
pub(crate) fn session_server() -> DisplayServer {
    if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty()) {
        return DisplayServer::Wayland;
    }
    match std::env::var("DISPLAY")
        .ok()
        .filter(|display| !display.is_empty())
    {
        Some(display) => DisplayServer::X11(display),
        None => DisplayServer::None,
    }
}
