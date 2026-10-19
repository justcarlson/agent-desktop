#![expect(dead_code, reason = "stub the owning lane replaces")]

pub(crate) const MARKER_ENV: &str = "AGENT_DESKTOP_SELECTION_OWNER";
pub(crate) const BUILD_IDENTITY_ENV: &str = "AGENT_DESKTOP_SELECTION_BUILD";
pub(crate) const PAYLOAD_FD_ENV: &str = "AGENT_DESKTOP_SELECTION_PAYLOAD_FD";
pub(crate) const DISPLAY_ENV: &str = "AGENT_DESKTOP_SELECTION_DISPLAY";

pub(crate) fn build_identity() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

pub fn run_from_env() -> i32 {
    2
}
