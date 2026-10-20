use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

pub(crate) const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PortalAnswer {
    Granted { uri: Option<String> },
    Refused { code: u32 },
}

/// A subscription to one request's `Response` signal. It exists before the
/// call that creates the request, so a response that comes first is kept.
pub(crate) trait Pending {
    /// Closes the request so the portal drops its dialog and its result.
    fn cancel(self: Box<Self>);
    fn wait(self: Box<Self>, deadline: Deadline) -> Result<PortalAnswer, AdapterError>;
}

/// The portal's session-bus calls over plain values.
pub(crate) trait PortalWire {
    fn unique_name(&self) -> String;
    fn subscribe(
        &self,
        request_path: &str,
        deadline: Deadline,
    ) -> Result<Box<dyn Pending>, AdapterError>;
    fn call_screenshot(&self, token: &str, deadline: Deadline) -> Result<String, AdapterError>;
}

pub(crate) fn request_path(unique_name: &str, token: &str) -> String {
    let sender: String = unique_name
        .trim_start_matches(':')
        .chars()
        .map(|c| if c == '.' { '_' } else { c })
        .collect();
    format!("/org/freedesktop/portal/desktop/request/{sender}/{token}")
}

fn refused(code: u32) -> AdapterError {
    if code == 1 {
        return AdapterError::new(
            ErrorCode::PermDenied,
            "The desktop portal did not allow the screenshot (response 1)",
        )
        .with_suggestion(
            "A person chose Deny or closed the dialog. Allow screenshots in the dialog the desktop shows; if a Deny was remembered, reset it in the desktop's privacy settings first",
        );
    }
    AdapterError::new(
        ErrorCode::PermDenied,
        format!("The desktop portal ended the screenshot request without a result (response {code})"),
    )
    .with_suggestion(
        "The portal shows one screenshot dialog at a time, so a dialog left open by an earlier call is the usual cause: answer it or wait up to 25 seconds for the portal to close it, then retry. Other causes: a person chose Deny, or the portal backend failed (check that xdg-desktop-portal and its backend for this desktop run)",
    )
}

fn timed_out(error: AdapterError) -> AdapterError {
    error.with_suggestion(
        "The desktop asks to allow screenshots in a dialog; a person must approve it before the time limit",
    )
}

fn no_uri() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The desktop portal allowed the screenshot but named no file",
    )
}

fn other_handle(handle: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The desktop portal answered on a request this call did not subscribe to",
    )
    .with_platform_detail(handle)
}

/// Subscribes to the request's response, then calls `Screenshot`, then waits.
/// The order is the point: the signal can arrive before the call returns.
pub(crate) fn screenshot_uri(
    wire: &dyn PortalWire,
    token: &str,
    deadline: Deadline,
) -> Result<String, AdapterError> {
    let path = request_path(&wire.unique_name(), token);
    let pending = wire.subscribe(&path, deadline)?;
    let handle = match wire.call_screenshot(token, deadline) {
        Ok(handle) => handle,
        Err(error) => {
            pending.cancel();
            return Err(error);
        }
    };
    if handle != path {
        pending.cancel();
        return Err(other_handle(&handle));
    }
    match pending.wait(deadline) {
        Ok(PortalAnswer::Granted { uri: Some(uri) }) => Ok(uri),
        Ok(PortalAnswer::Granted { uri: None }) => Err(no_uri()),
        Ok(PortalAnswer::Refused { code }) => Err(refused(code)),
        Err(error) if error.code == ErrorCode::Timeout => Err(timed_out(error)),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "wayland_portal_call_tests.rs"]
mod tests;
