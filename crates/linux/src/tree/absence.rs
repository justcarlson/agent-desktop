use std::io::ErrorKind;

use agent_desktop_core::{AdapterError, ErrorCode};

use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN};

pub(crate) const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
pub(crate) const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
pub(crate) const UNKNOWN_INTERFACE: &str = "org.freedesktop.DBus.Error.UnknownInterface";

/// What one failed call says on its own. The names of the suspect family are
/// answered both by destroyed objects and by live ones for interfaces they do
/// not implement (L17-2), so on their own they prove nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Failure {
    AppGone,
    Suspect,
    TimedOut,
    Other,
}

/// What a node's own `Accessible.GetRole` says about it. The call is
/// implemented by every live node of every toolkit (L17-11), so its failure in
/// the suspect family is the corroboration a destroyed object needs. A
/// destroyed GTK3 or Chromium object keeps answering it for about 16 s
/// (L17-6, L17-8): an answering node is alive to its own reads only, and
/// presence is decided by the parent's child list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Liveness {
    Alive,
    TimedOut,
    ObjectGone,
    AppGone,
}

/// What a failed read of one field means for the read of its node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    AppGone,
    ObjectGone,
    Failed,
}

pub(crate) fn failure_of_name(name: &str) -> Failure {
    match name {
        SERVICE_UNKNOWN | NAME_HAS_NO_OWNER => Failure::AppGone,
        UNKNOWN_OBJECT | UNKNOWN_METHOD | UNKNOWN_INTERFACE => Failure::Suspect,
        _ => Failure::Other,
    }
}

pub(crate) fn failure(error: &zbus::Error) -> Failure {
    match error {
        zbus::Error::MethodError(name, _, _) => failure_of_name(name.as_str()),
        zbus::Error::InputOutput(io) | zbus::Error::Connection(io, _)
            if io.kind() == ErrorKind::TimedOut =>
        {
            Failure::TimedOut
        }
        _ => Failure::Other,
    }
}

impl Liveness {
    /// The liveness a `GetRole` outcome shows: `None` when the call answered.
    pub(crate) fn of(role_error: Option<&zbus::Error>) -> Self {
        match role_error.map(failure) {
            None | Some(Failure::Other) => Self::Alive,
            Some(Failure::AppGone) => Self::AppGone,
            Some(Failure::Suspect) => Self::ObjectGone,
            Some(Failure::TimedOut) => Self::TimedOut,
        }
    }
}

pub(crate) fn read_failure(failure: Failure, liveness: Liveness) -> ReadFailure {
    match (failure, liveness) {
        (Failure::AppGone, _) | (_, Liveness::AppGone) => ReadFailure::AppGone,
        (_, Liveness::ObjectGone) => ReadFailure::ObjectGone,
        _ => ReadFailure::Failed,
    }
}

pub(crate) fn app_left(error: &zbus::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        "The application left the accessibility bus",
    )
    .with_platform_detail(error.to_string())
}

pub(crate) fn element_gone(error: &zbus::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::ElementNotFound,
        "The accessible object no longer exists",
    )
    .with_platform_detail(error.to_string())
}

/// The error that ends a node read when a call shows the object or its
/// application gone, or `None` when the failure stays on its field.
pub(crate) fn gone_error(error: &zbus::Error, liveness: Liveness) -> Option<AdapterError> {
    match read_failure(failure(error), liveness) {
        ReadFailure::AppGone => Some(app_left(error)),
        ReadFailure::ObjectGone => Some(element_gone(error)),
        ReadFailure::Failed => None,
    }
}

#[cfg(test)]
#[path = "absence_tests.rs"]
mod tests;
