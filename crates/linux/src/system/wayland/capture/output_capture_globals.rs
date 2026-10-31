use crate::system::wayland::toplevel::advertised_global::AdvertisedGlobal;

/// The globals one output capture binds: shared memory, the screencopy
/// manager, and every output at a version that announces its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputCaptureGlobals {
    pub(crate) shm: AdvertisedGlobal,
    pub(crate) screencopy: AdvertisedGlobal,
    pub(crate) outputs: Vec<AdvertisedGlobal>,
}
