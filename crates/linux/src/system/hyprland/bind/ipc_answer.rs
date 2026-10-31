/// What one IPC endpoint did within the deadline: answered for a compositor
/// process, refused or vanished, or accepted and stayed silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpcAnswer {
    Live(u32),
    Gone,
    Silent,
}
