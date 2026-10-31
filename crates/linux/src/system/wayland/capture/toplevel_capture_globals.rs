use crate::system::wayland::global::Global;

/// The globals one toplevel capture binds: shared memory, the source
/// manager that turns a foreign toplevel handle into a capture source, and
/// the copy manager that captures it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToplevelCaptureGlobals {
    pub(crate) shm: Global,
    pub(crate) sources: Global,
    pub(crate) copy: Global,
}
