use agent_desktop_core::{AdapterError, Deadline};
use zbus::Connection;

use super::node_read::call;
use super::{AtspiElement, COMPONENT_INTERFACE, Extents};
use crate::system::{SharedBus, within};

pub(crate) const WINDOW_COORDINATES: u32 = 1;

/// One `GetExtents` in window coordinates. Any failed or malformed reply is
/// `None`: the position is evidence only when it was read.
pub(crate) async fn read_window_extents_async(
    connection: &Connection,
    element: &AtspiElement,
) -> Option<Extents> {
    let reply = call(
        connection,
        element,
        COMPONENT_INTERFACE,
        "GetExtents",
        &(WINDOW_COORDINATES,),
    )
    .await
    .ok()?;
    let (x, y, width, height) = reply.body().deserialize::<(i32, i32, i32, i32)>().ok()?;
    Some(Extents {
        x,
        y,
        width,
        height,
    })
}

/// An element's rectangle in window coordinates, or `None` when it cannot be
/// read inside the deadline.
pub(crate) fn read_window_extents(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Option<Extents> {
    let connection = bus.get(deadline).ok()?;
    within(deadline, async {
        Ok::<_, AdapterError>(read_window_extents_async(connection.inner(), element).await)
    })
    .ok()
    .flatten()
}
