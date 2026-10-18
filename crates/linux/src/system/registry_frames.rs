use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};
use futures_util::stream::{self, StreamExt};

use super::bus_slice::within_slice;
use super::{SharedBus, within};
use crate::tree::{AtspiElement, RawNode, list_children, read_node};

const CHILDREN_SLICE: Duration = Duration::from_millis(300);
const APPS_IN_FLIGHT: usize = 16;
const FRAMES_IN_FLIGHT: usize = 8;

/// Reads the children of each application root in full. An application that
/// does not list its children in time, or whose frame is gone, contributes
/// no windows; it is not an error.
pub(crate) fn read_frames(
    bus: &SharedBus,
    roots: &[AtspiElement],
    deadline: Deadline,
) -> Result<Vec<Vec<RawNode>>, AdapterError> {
    let shared = bus.get(deadline)?;
    within(deadline, async {
        let connection = shared.inner();
        let reads = roots.iter().map(|root| async move {
            let listed = within_slice(CHILDREN_SLICE, list_children(connection, root)).await;
            let Some(Ok(children)) = listed else {
                return Vec::new();
            };
            stream::iter(children.into_iter().map(|(bus_name, path)| async move {
                let frame = AtspiElement::new(bus_name, path, root.pid());
                read_node(connection, &frame).await.ok()
            }))
            .buffered(FRAMES_IN_FLIGHT)
            .filter_map(|node| async move { node })
            .collect::<Vec<_>>()
            .await
        });
        Ok(stream::iter(reads)
            .buffered(APPS_IN_FLIGHT)
            .collect::<Vec<_>>()
            .await)
    })
}
