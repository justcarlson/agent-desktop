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
