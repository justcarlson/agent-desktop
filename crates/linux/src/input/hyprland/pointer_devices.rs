use wayland_client::QueueHandle;
use wayland_client::protocol::{wl_output, wl_seat};
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

use super::pointer_state::PointerState;

/// The seat, the virtual pointer manager, every named output and the
/// pointer made so far for each output. A pointer bound to an output maps
/// absolute motion onto that output alone.
pub(crate) struct PointerDevices {
    pub(crate) seat: wl_seat::WlSeat,
    pub(crate) manager: ZwlrVirtualPointerManagerV1,
    pub(crate) outputs: Vec<(String, wl_output::WlOutput)>,
    pub(crate) pointers: Vec<(String, ZwlrVirtualPointerV1)>,
}

impl PointerDevices {
    /// The pointer bound to `output`, made on first use; `None` for an
    /// output the compositor did not name.
    pub(crate) fn pointer_for(
        &mut self,
        output: &str,
        handle: &QueueHandle<PointerState>,
    ) -> Option<(&ZwlrVirtualPointerV1, bool)> {
        if let Some(index) = self.pointers.iter().position(|(name, _)| name == output) {
            return self
                .pointers
                .get(index)
                .map(|(_, pointer)| (pointer, false));
        }
        let (_, bound) = self.outputs.iter().find(|(name, _)| name == output)?;
        let pointer = self.manager.create_virtual_pointer_with_output(
            Some(&self.seat),
            Some(bound),
            handle,
            (),
        );
        self.pointers.push((output.to_owned(), pointer));
        self.pointers.last().map(|(_, pointer)| (pointer, true))
    }

    /// The pointer already made for `output`.
    pub(crate) fn made(&self, output: &str) -> Option<&ZwlrVirtualPointerV1> {
        self.pointers
            .iter()
            .find(|(name, _)| name == output)
            .map(|(_, pointer)| pointer)
    }
}

impl Drop for PointerDevices {
    fn drop(&mut self) {
        for (_, pointer) in self.pointers.drain(..) {
            pointer.destroy();
        }
    }
}
