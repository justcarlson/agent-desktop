use crate::system::wayland::global::Global;

/// The globals one output capture binds: shared memory, the screencopy
/// manager, and every output at a version that announces its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputCaptureGlobals {
    pub(crate) shm: Global,
    pub(crate) screencopy: Global,
    pub(crate) outputs: Vec<Global>,
}
