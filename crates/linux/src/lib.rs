#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod actions;
mod adapter;
mod input;
mod notifications;
mod system;
mod tree;

pub use adapter::LinuxAdapter;
pub use input::clipboard::owner::owner_entry::run_from_env as clipboard_owner_from_env;
pub use input::wayland::kept_session::end as end_input_session;
pub use input::x11::signal_release::enable as release_input_on_signals;

#[allow(dead_code)]
pub(crate) fn ltrace(msg: impl AsRef<str>) {
    use std::io::Write;
    if let Ok(path) = std::env::var("AD_LISTING_TRACE") {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let _ = writeln!(f, "{t} pid={} {}", std::process::id(), msg.as_ref());
        }
    }
}
