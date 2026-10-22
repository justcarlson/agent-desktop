use std::future::Future;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};
use async_io::Timer;
use futures_lite::future::or;

/// Drives `future` to completion, giving up with the deadline's timeout error
/// once the caller's remaining budget is spent.
pub(crate) fn within<T>(
    deadline: Deadline,
    future: impl Future<Output = Result<T, AdapterError>>,
) -> Result<T, AdapterError> {
    let slice = deadline.remaining_slice(Duration::MAX)?;
    zbus::block_on(or(future, async move {
        Timer::after(slice).await;
        Err(deadline.timeout_error())
    }))
}
