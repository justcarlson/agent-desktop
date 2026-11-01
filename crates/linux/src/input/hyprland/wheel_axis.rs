/// The wheel axis of one `axis_discrete` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WheelAxis {
    Vertical,
    Horizontal,
}
