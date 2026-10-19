use agent_desktop_core::{
    AdapterError, Deadline, ErrorCode, ImageBuffer, ScreenshotTarget, WindowInfo,
};

use super::SharedBus;

pub(crate) fn screenshot(
    _bus: &SharedBus,
    _target: ScreenshotTarget,
    _deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Screenshots are not built yet",
    ))
}

pub(crate) fn screenshot_window_frame(
    _bus: &SharedBus,
    _window: &WindowInfo,
    _deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    Err(AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Screenshots are not built yet",
    ))
}
