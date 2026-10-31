use super::frame_record::FrameRecord;
use super::screencopy_record::ScreencopyRecord;
use super::session_record::SessionRecord;
use crate::system::wayland::toplevel::advertised_global::AdvertisedGlobal;

/// Everything one capture queue has been told: the registry globals, how
/// many sync callbacks completed, the toplevel capture session, the
/// screencopy offers, the frame, and the name each bound output announced
/// keyed by its registry name.
#[derive(Debug, Default)]
pub(crate) struct CaptureState {
    pub(crate) globals: Vec<AdvertisedGlobal>,
    pub(crate) synced: u32,
    pub(crate) session: SessionRecord,
    pub(crate) screencopy: ScreencopyRecord,
    pub(crate) frame: FrameRecord,
    pub(crate) outputs: Vec<(u32, String)>,
}
