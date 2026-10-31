/// Why a Hyprland IPC request has no reply: nothing serves the socket or it
/// closed without a reply, it did not answer before the deadline, or the
/// reply grew past the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpcFault {
    Gone,
    Silent,
    Oversized,
}
