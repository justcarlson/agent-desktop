//! Whether a frame was produced after it was requested. The compositor
//! stamps each frame with its presentation time on the monotonic clock;
//! the capture reads that clock just before it asks for the frame. Damage
//! says nothing about age: a static window's first frame is fully damaged
//! and still new.

use std::time::Duration;

/// The monotonic clock the compositor stamps frames with.
pub(crate) fn monotonic_now() -> Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    Duration::new(
        u64::try_from(now.tv_sec).unwrap_or(0),
        u32::try_from(now.tv_nsec).unwrap_or(0),
    )
}

/// Whether a frame presented at `presented` is no older than `requested`.
/// A frame without a presentation time is not proven fresh.
pub(crate) fn is_fresh(requested: Duration, presented: Option<Duration>) -> bool {
    presented.is_some_and(|presented| presented >= requested)
}

#[cfg(test)]
#[path = "frame_freshness_tests.rs"]
mod tests;
