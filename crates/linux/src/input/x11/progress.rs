use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Debug, Default)]
pub(crate) struct Progress {
    cancelled: AtomicBool,
    delivered: AtomicU32,
}

impl Progress {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn record(&self, delivered: u32) {
        self.delivered.store(delivered, Ordering::SeqCst);
    }

    pub(crate) fn delivered(&self) -> u32 {
        self.delivered.load(Ordering::SeqCst)
    }
}
