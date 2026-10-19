#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    pub(crate) delivered_events: u32,
    pub(crate) mechanism: &'static str,
}
