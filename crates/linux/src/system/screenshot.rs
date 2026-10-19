use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, ImageBuffer, ScreenshotTarget, WindowInfo,
};

use super::SharedBus;
use super::capture::wayland_portal::capture_wayland;
use super::capture::window_context::{BusLookup, CaptureEnv, real_decode};
use super::capture::{display, png_encode::encode_png, window};
use super::route::{Route, current};
use super::x11::DisplayReader;
use super::x11::grab::DisplayGrabber;

fn not_delivered(error: AdapterError) -> AdapterError {
    if error.disposition == DeliverySemantics::Unknown {
        error.with_disposition(DeliverySemantics::not_delivered())
    } else {
        error
    }
}

fn wayland_window() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "A single window cannot be captured on a Wayland session",
    )
    .with_suggestion("Capture the screen instead, or run the application in an X11 session")
}

/// Captures `target` by the session's route. A window is only ever captured on
/// X11; every Wayland window request is refused before any X11 call.
pub(crate) fn capture_with(
    env: &CaptureEnv,
    route: &Route,
    target: ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    match (route, target) {
        (Route::Wayland, ScreenshotTarget::ExactWindow(_)) => Err(wayland_window()),
        (Route::Wayland, target) => capture_wayland(&target, deadline),
        (Route::X11(_), ScreenshotTarget::ExactWindow(window)) => {
            window::capture(env, &window, false, deadline)
        }
        (Route::X11(_), target) => display::capture(env, &target, deadline),
    }
    .map_err(not_delivered)
}

/// Captures the window for the debug frame, refusing a window whose drawables
/// match no reported bounds rather than returning an image core would reject.
pub(crate) fn capture_frame_with(
    env: &CaptureEnv,
    route: &Route,
    window: &WindowInfo,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    match route {
        Route::Wayland => Err(wayland_window()),
        Route::X11(_) => window::capture(env, window, true, deadline),
    }
    .map_err(not_delivered)
}

fn with_env<T>(
    bus: &SharedBus,
    run: impl FnOnce(&CaptureEnv) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let lookup = BusLookup { bus };
    let env = CaptureEnv {
        grabber: &DisplayGrabber,
        stacking: &DisplayReader,
        lookup: &lookup,
        decode: real_decode,
        encode: encode_png,
    };
    run(&env)
}

pub(crate) fn screenshot(
    bus: &SharedBus,
    target: ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    let route = current().map_err(not_delivered)?;
    with_env(bus, |env| capture_with(env, &route, target, deadline))
}

pub(crate) fn screenshot_window_frame(
    bus: &SharedBus,
    window: &WindowInfo,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    let route = current().map_err(not_delivered)?;
    with_env(bus, |env| capture_frame_with(env, &route, window, deadline))
}

#[cfg(test)]
#[path = "screenshot_tests.rs"]
mod tests;
