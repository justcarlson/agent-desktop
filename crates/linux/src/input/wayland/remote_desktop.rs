use std::io::ErrorKind;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::key_session::KeySession;
use crate::system::session_bus::{SessionBus, is_transport_failure};
use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN, within};

const SERVICE: &str = "org.gnome.Mutter.RemoteDesktop";
const PATH: &str = "/org/gnome/Mutter/RemoteDesktop";
const SESSION_INTERFACE: &str = "org.gnome.Mutter.RemoteDesktop.Session";
const ACCESS_DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";

/// A typed view of Mutter's RemoteDesktop service over the session bus
/// connection. It never holds the accessibility connection.
pub(crate) struct RemoteDesktop {
    connection: Connection,
    session: Option<OwnedObjectPath>,
    started: bool,
}

impl RemoteDesktop {
    pub(crate) fn open(deadline: Deadline) -> Result<Self, AdapterError> {
        Ok(Self::over(SessionBus::get(deadline)?))
    }

    pub(crate) fn over(connection: Connection) -> Self {
        Self {
            connection,
            session: None,
            started: false,
        }
    }

    fn session_path(&self) -> Result<&str, AdapterError> {
        self.session
            .as_ref()
            .map(|path| path.as_str())
            .ok_or_else(|| AdapterError::internal("The keyboard session was not created"))
    }

    fn failed(&self, error: &zbus::Error) -> AdapterError {
        if is_transport_failure(error) {
            SessionBus::forget(&self.connection);
        }
        classify_call(error)
    }

    fn forget_on_timeout(&self, error: AdapterError) -> AdapterError {
        if error.code == ErrorCode::Timeout {
            SessionBus::forget(&self.connection);
        }
        error
    }

    fn call_session(&self, member: &'static str, deadline: Deadline) -> Result<(), AdapterError> {
        let path = self.session_path()?;
        within(deadline, async {
            self.connection
                .call_method(Some(SERVICE), path, Some(SESSION_INTERFACE), member, &())
                .await
                .map(|_| ())
                .map_err(|error| self.failed(&error))
        })
        .map_err(|error| self.forget_on_timeout(error))
    }
}

impl RemoteDesktop {
    pub(crate) fn pointer_motion_relative(
        &mut self,
        dx: f64,
        dy: f64,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let path = self.session_path()?;
        within(deadline, async {
            self.connection
                .call_method(
                    Some(SERVICE),
                    path,
                    Some(SESSION_INTERFACE),
                    "NotifyPointerMotionRelative",
                    &(dx, dy),
                )
                .await
                .map(|_| ())
                .map_err(|error| classify_call(&error))
        })
    }

    pub(crate) fn pointer_button(
        &mut self,
        button: u32,
        down: bool,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let path = self.session_path()?;
        let code = i32::try_from(button)
            .map_err(|_| AdapterError::internal("The pointer button code is out of range"))?;
        within(deadline, async {
            self.connection
                .call_method(
                    Some(SERVICE),
                    path,
                    Some(SESSION_INTERFACE),
                    "NotifyPointerButton",
                    &(code, down),
                )
                .await
                .map(|_| ())
                .map_err(|error| classify_call(&error))
        })
    }
}

impl KeySession for RemoteDesktop {
    fn create(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let path = within(deadline, async {
            let reply = self
                .connection
                .call_method(Some(SERVICE), PATH, Some(SERVICE), "CreateSession", &())
                .await
                .map_err(|error| self.failed(&error))?;
            reply
                .body()
                .deserialize::<OwnedObjectPath>()
                .map_err(|error| classify_call(&error))
        })
        .map_err(|error| self.forget_on_timeout(error))
        .map_err(creation_failure)?;
        self.session = Some(path);
        Ok(())
    }

    fn start(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.call_session("Start", deadline)?;
        self.started = true;
        Ok(())
    }

    fn resumed(&self) -> bool {
        self.started && self.session.is_some()
    }

    fn retain(&self) -> bool {
        true
    }

    fn keysym(&mut self, keysym: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        let path = self.session_path()?;
        within(deadline, async {
            self.connection
                .call_method(
                    Some(SERVICE),
                    path,
                    Some(SESSION_INTERFACE),
                    "NotifyKeyboardKeysym",
                    &(keysym, down),
                )
                .await
                .map(|_| ())
                .map_err(|error| self.failed(&error))
        })
        .map_err(|error| self.forget_on_timeout(error))
    }

    fn stop(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let stopped = self.call_session("Stop", deadline);
        self.session = None;
        self.started = false;
        if stopped.is_err() {
            SessionBus::forget(&self.connection);
        }
        stopped
    }
}

pub(crate) fn semantic_action_refusal(message: impl Into<String>) -> AdapterError {
    AdapterError::new(ErrorCode::ActionNotSupported, message)
        .with_suggestion("Use the semantic action on the element instead of physical input")
        .with_details(json!({ "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered())
}

fn creation_failure(error: AdapterError) -> AdapterError {
    match error.code {
        ErrorCode::Timeout => AdapterError::timeout(
            "The compositor input service did not answer session creation in time",
        )
        .with_suggestion(
            "The desktop session is busy or its input service is unreachable; retry, or use the semantic action",
        )
        .with_details(json!({ "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered()),
        _ => error,
    }
}

pub(crate) fn classify_call(error: &zbus::Error) -> AdapterError {
    match error {
        zbus::Error::InputOutput(io) | zbus::Error::Connection(io, _)
            if io.kind() == ErrorKind::TimedOut =>
        {
            AdapterError::timeout("The compositor did not answer the keyboard request in time")
                .with_platform_detail(error.to_string())
        }
        zbus::Error::MethodError(name, _, _)
            if matches!(
                name.as_str(),
                SERVICE_UNKNOWN | NAME_HAS_NO_OWNER | ACCESS_DENIED
            ) =>
        {
            semantic_action_refusal(
                "Keyboard input through the compositor is not available in this session",
            )
            .with_platform_detail(error.to_string())
        }
        _ => AdapterError::new(
            ErrorCode::ActionFailed,
            "The compositor rejected the keyboard request",
        )
        .with_suggestion("Retry the command; report the platform detail if it persists")
        .with_platform_detail(error.to_string()),
    }
}

#[cfg(test)]
#[path = "remote_desktop_tests.rs"]
mod tests;
