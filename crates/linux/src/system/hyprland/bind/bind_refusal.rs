use std::path::PathBuf;

use agent_desktop_core::AdapterError;

use crate::system::display::display_server::{DisplayCause, display_cause_error};
use crate::system::wayland::wayland_fault::WaylandFault;

/// Why no Hyprland instance may be bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BindRefusal {
    Ambiguous { signatures: Vec<String> },
    Mismatch { detail: String },
    Silent { socket: PathBuf },
}

impl BindRefusal {
    /// The display error a refusal reports, with its cause in
    /// `details.display_unavailable`.
    pub(crate) fn to_error(&self) -> AdapterError {
        match self {
            Self::Ambiguous { signatures } => {
                display_cause_error(&DisplayCause::HyprlandAmbiguous {
                    signatures: signatures.clone(),
                })
            }
            Self::Mismatch { detail } => display_cause_error(&DisplayCause::HyprlandMismatch {
                detail: detail.clone(),
            }),
            Self::Silent { socket } => WaylandFault::Silent.into_error(socket),
        }
    }
}

#[cfg(test)]
#[path = "bind_refusal_tests.rs"]
mod tests;
