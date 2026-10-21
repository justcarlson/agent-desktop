#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatcherRead {
    NoOwner,
    Items,
}
