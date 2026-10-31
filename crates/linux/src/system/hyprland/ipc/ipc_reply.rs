/// One IPC reply and the pid the kernel names as the socket's server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IpcReply {
    pub(crate) pid: u32,
    pub(crate) body: Vec<u8>,
}
