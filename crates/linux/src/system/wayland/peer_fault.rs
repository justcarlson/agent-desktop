/// Why a Unix socket gave no peer: nothing listens there, or the listener's
/// queue is full so a connect would block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeerFault {
    Gone,
    Busy,
}
