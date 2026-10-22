use agent_desktop_core::NotificationInfo;

use crate::tree::AtspiElement;

/// One entry as a read found it: what it shows and the accessibility object
/// that shows it. The object, not the text, tells two entries apart, so two
/// notifications with the same words stay two.
#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub(crate) info: NotificationInfo,
    pub(crate) node: AtspiElement,
}

pub(crate) fn same_object(left: &AtspiElement, right: &AtspiElement) -> bool {
    left.bus_name() == right.bus_name() && left.path() == right.path()
}

/// Whether the object still stands in a reading of the list.
pub(crate) fn holds(list: &[Seen], node: &AtspiElement) -> bool {
    list.iter().any(|seen| same_object(&seen.node, node))
}
