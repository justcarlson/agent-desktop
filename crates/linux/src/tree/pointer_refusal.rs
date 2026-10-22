use agent_desktop_core::{AdapterError, Deadline, NativeHandle};

use super::atspi_element;
use super::live_project::pointer_rect;
use super::live_read::{NodeReader, read_live, with_reader};
use super::read_purpose::ReadPurpose;
use super::resolve_errors::window_tied;
use crate::system::SharedBus;

/// The refusal for a target whose window shares its title with a twin that
/// nothing tells apart, so no pointer position can be proved for it.
pub(crate) fn pointer_refusal_via<R: NodeReader>(
    reader: &mut R,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Result<Option<AdapterError>, AdapterError> {
    let node = read_live(reader, handle)?;
    let element = atspi_element(handle)?;
    if reader.global_position(element, deadline) {
        return Ok(None);
    }
    let placed = reader.placement(element, deadline);
    Ok((placed.tie.is_some() && pointer_rect(&node.read, &placed).is_none()).then(window_tied))
}

/// [`pointer_refusal_via`] on the bus. A read that fails proves nothing, so it
/// answers no refusal and leaves the failure to the read that needs it.
pub(crate) fn pointer_position_refusal(
    bus: &SharedBus,
    handle: &NativeHandle,
    deadline: Deadline,
) -> Option<AdapterError> {
    with_reader(
        bus,
        handle,
        deadline,
        ReadPurpose::Bounds,
        |reader, handle| pointer_refusal_via(reader, handle, deadline),
    )
    .ok()
    .flatten()
}
