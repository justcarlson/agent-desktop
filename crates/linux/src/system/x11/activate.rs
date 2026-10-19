use super::activate_error::ActivateError;

/// Asks the window manager to make `client` the active window.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn activate(_client: u32, _source: u32, _timestamp: u32) -> Result<(), ActivateError> {
    Err(ActivateError::NotYetImplemented)
}
