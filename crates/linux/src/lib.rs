#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod actions;
mod adapter;
mod input;
mod system;
mod tree;

pub use adapter::LinuxAdapter;

/// Makes an input call that a SIGTERM, SIGINT or SIGHUP interrupts release the
/// keys and buttons it holds before the signal takes effect. The command-line
/// binary turns it on; a library host never does and keeps its own handlers.
pub fn release_input_on_signals() {
    input::x11::signal_release::enable();
}

/// Ends the keyboard session a Wayland delivery left open for the next one.
/// The command-line binary calls it before the process ends: a session left to
/// the compositor's disconnect handling is torn down late, and that teardown
/// can take the keyboard focus from the session of the next process.
pub fn end_input_session() {
    input::wayland::kept_session::end();
}
