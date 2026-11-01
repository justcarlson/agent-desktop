use agent_desktop_core::AdapterError;

/// What a physical delivery sent: the events it confirmed and whether they
/// went through a Wayland virtual keyboard rather than the backend's default
/// route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    pub(crate) delivered_events: u32,
    pub(crate) virtual_keyboard: bool,
}

pub(crate) type Gate<'a> = &'a mut dyn FnMut() -> Result<(), AdapterError>;
