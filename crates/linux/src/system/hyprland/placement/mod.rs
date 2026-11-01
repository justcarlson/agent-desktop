//! Where a bound window's element sits in the global layout: the window's
//! compositor origin plus the element's window-relative rectangle, in the
//! logical pixels `list-displays` reports.

pub(crate) mod bound_layout;
pub(crate) mod layout_rect;
pub(crate) mod placement;
