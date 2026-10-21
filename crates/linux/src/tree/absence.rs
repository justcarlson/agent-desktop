use std::io::ErrorKind;

use super::failure::Failure;
use super::liveness::Liveness;
use super::read_failure::ReadFailure;

use agent_desktop_core::{AdapterError, ErrorCode};

use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN};

pub(crate) const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
pub(crate) const UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";
pub(crate) const UNKNOWN_INTERFACE: &str = "org.freedesktop.DBus.Error.UnknownInterface";

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
