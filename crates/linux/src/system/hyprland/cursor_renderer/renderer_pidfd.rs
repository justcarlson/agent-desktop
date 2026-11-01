use std::os::fd::OwnedFd;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};

/// A pidfd for one process. A signal sent through it reaches that process
/// or none: never a later process that reuses the pid.
pub(crate) struct RendererPidfd {
    fd: OwnedFd,
}

impl RendererPidfd {
    /// A pidfd for `pid`, or `None` when no such process exists or pidfds
    /// are unavailable.
    pub(crate) fn open(pid: u32) -> Option<Self> {
        let pid = Pid::from_raw(i32::try_from(pid).ok()?)?;
        pidfd_open(pid, PidfdFlags::empty())
            .ok()
            .map(|fd| Self { fd })
    }

    /// Whether the process has exited, waiting up to `wait` for it.
    pub(crate) fn wait_for_exit(&self, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Ok(timeout) = Timespec::try_from(remaining) else {
                return false;
            };
            let mut fds = [PollFd::new(&self.fd, PollFlags::IN)];
            match rustix::event::poll(&mut fds, Some(&timeout)) {
                Ok(0) => return false,
                Ok(_) => return true,
                Err(rustix::io::Errno::INTR) if !remaining.is_zero() => continue,
                Err(_) => return false,
            }
        }
    }

    /// Sends `signal` through the pidfd.
    pub(crate) fn signal(&self, signal: Signal) -> bool {
        pidfd_send_signal(&self.fd, signal).is_ok()
    }
}
