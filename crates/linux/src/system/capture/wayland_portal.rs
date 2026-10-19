use agent_desktop_core::{AdapterError, Deadline, ErrorCode, ImageBuffer, ScreenshotTarget};

pub(crate) fn capture_wayland(
    _target: &ScreenshotTarget,
    _deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Wayland capture is not built yet",
    ))
}
