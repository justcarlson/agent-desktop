use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

/// Why `focus_window` ended without the window being active. Every cause
/// but a post-send one is decided before anything is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    WaylandSession,
    NoDisplay,
    NoWindowManager,
    X11Unavailable,
    WindowMinimized,
    WindowHidden,
    WindowNotMapped,
    AmbiguousWindow,
    ActivationNotSent,
    ActivationNotGranted,
    ActivationRefused,
    FrameNotActive,
}

impl Cause {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::WaylandSession => "wayland_session",
            Self::NoDisplay => "no_display",
            Self::NoWindowManager => "no_window_manager",
            Self::X11Unavailable => "x11_unavailable",
            Self::WindowMinimized => "window_minimized",
            Self::WindowHidden => "window_hidden",
            Self::WindowNotMapped => "window_not_mapped",
            Self::AmbiguousWindow => "ambiguous_window",
            Self::ActivationNotSent => "activation_not_sent",
            Self::ActivationNotGranted => "activation_not_granted",
            Self::ActivationRefused => "activation_refused",
            Self::FrameNotActive => "frame_not_active",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::WaylandSession => {
                "A Wayland session lets no client activate another client's window"
            }
            Self::NoDisplay => "No display server answered, so the window cannot be activated",
            Self::NoWindowManager => {
                "No window manager is running, so nothing can activate the window"
            }
            Self::X11Unavailable => "The display server did not describe the window in time",
            Self::WindowMinimized => "The window is minimized and is not activated from here",
            Self::WindowHidden => "The window manager reports the window hidden",
            Self::WindowNotMapped => "The window is not mapped on the display",
            Self::AmbiguousWindow => {
                "More than one display window matches this window, so none is activated"
            }
            Self::ActivationNotSent => {
                "The activation request could not be sent to the window manager"
            }
            Self::ActivationNotGranted => "The window manager did not grant focus to the window",
            Self::ActivationRefused => "The window manager left another window active",
            Self::FrameNotActive => {
                "The window manager made the window active but the application does not report it active"
            }
        }
    }

    fn suggestion(self) -> &'static str {
        match self {
            Self::ActivationNotGranted => {
                "The window manager ignores activation of a window that has never held focus; focus the window once yourself, then retry"
            }
            Self::WindowMinimized | Self::WindowHidden => {
                "Restore the window with 'restore-window', then retry"
            }
            Self::WaylandSession | Self::NoDisplay | Self::NoWindowManager => {
                "Run the desktop session on X11 with a window manager, or focus the window yourself"
            }
            _ => "Run 'list-windows' to confirm the window, then retry",
        }
    }
}

pub(crate) fn refusal(cause: Cause) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, cause.message())
        .with_suggestion(cause.suggestion())
        .with_details(json!({
            "physical_delivery_started": false,
            "cause": cause.as_str(),
        }))
        .with_disposition(DeliverySemantics::not_delivered())
}
