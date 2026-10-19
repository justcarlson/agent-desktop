use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::damage::{ConnectionExt as _, Damage, ReportLevel};
use x11rb::protocol::xproto::Window;
use x11rb::rust_connection::RustConnection;

const REDIRECT_WAIT: Duration = Duration::from_millis(100);
const REPAINT_WAIT: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(2);

/// Damage reports on one window, created before it is redirected so the
/// server's own first report is not missed.
pub(super) struct Repaint<'a> {
    connection: &'a RustConnection,
    damage: Damage,
}

impl<'a> Repaint<'a> {
    pub(super) fn watch(connection: &'a RustConnection, window: Window) -> Option<Self> {
        connection.damage_query_version(1, 1).ok()?.reply().ok()?;
        let damage = connection.generate_id().ok()?;
        connection
            .damage_create(damage, window, ReportLevel::NON_EMPTY)
            .ok()?
            .check()
            .ok()?;
        Some(Self { connection, damage })
    }

    fn damaged_within(&self, wait: Duration, deadline: Deadline) -> bool {
        let started = Instant::now();
        let limit = wait.min(deadline.remaining());
        loop {
            match self.connection.poll_for_event() {
                Ok(Some(Event::DamageNotify(event))) if event.damage == self.damage => return true,
                Ok(Some(_)) => continue,
                Ok(None) if started.elapsed() < limit => std::thread::sleep(POLL),
                _ => return false,
            }
        }
    }

    /// Waits for the redirect's own report and then for the client's repaint
    /// after it. A redirected pixmap read before that repaint can hold what
    /// was on the screen instead of the window's own paint. A window that
    /// does not repaint inside the wait is read as it stands.
    pub(super) fn settle(&self, deadline: Deadline) {
        let _ = self.connection.flush();
        if self.damaged_within(REDIRECT_WAIT, deadline) {
            let _ = self.connection.damage_subtract(self.damage, 0u32, 0u32);
            let _ = self.connection.flush();
            self.damaged_within(REPAINT_WAIT, deadline);
        }
    }
}

impl Drop for Repaint<'_> {
    fn drop(&mut self) {
        let _ = self.connection.damage_destroy(self.damage);
        let _ = self.connection.flush();
    }
}
