use std::future::Future;
use std::time::Duration;

use async_io::Timer;
use futures_lite::future::or;

/// Awaits `future` for at most `slice`; `None` means it did not answer in
/// time, which is how a stopped application looks from the bus.
pub(crate) async fn within_slice<T>(slice: Duration, future: impl Future<Output = T>) -> Option<T> {
    or(async { Some(future.await) }, async {
        Timer::after(slice).await;
        None
    })
    .await
}
