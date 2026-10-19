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
    AdapterError::new(
        ErrorCode::PermDenied,
        format!("The desktop portal did not allow the screenshot (response {code})"),
    )
    .with_suggestion("Allow screenshots in the dialog the desktop shows, then try again")
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
    let handle = wire.call_screenshot(token, deadline)?;
    if handle != path {
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
