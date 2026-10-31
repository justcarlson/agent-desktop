use crate::system::wayland::toplevel::advertised_global::AdvertisedGlobal;

/// The globals one toplevel capture binds: shared memory, the source
/// manager that turns a foreign toplevel handle into a capture source, and
/// the copy manager that captures it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToplevelCaptureGlobals {
    pub(crate) shm: AdvertisedGlobal,
    pub(crate) sources: AdvertisedGlobal,
    pub(crate) copy: AdvertisedGlobal,
}
