use agent_desktop_core::{AdapterError, ErrorCode, NativeHandle};

/// Identifies one accessible object on the accessibility bus: the unique bus
/// name of the owning application, the object path, and that application's
/// process id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AtspiElement {
    bus_name: String,
    path: String,
    pid: u32,
}

impl AtspiElement {
    pub(crate) fn new(bus_name: impl Into<String>, path: impl Into<String>, pid: u32) -> Self {
        Self {
            bus_name: bus_name.into(),
            path: path.into(),
            pid,
        }
    }

    pub(crate) fn bus_name(&self) -> &str {
        &self.bus_name
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }

    pub(crate) fn into_native_handle(self) -> NativeHandle {
        NativeHandle::new(self)
    }
}

/// Borrows the element carried by a core `NativeHandle`. A handle built by
/// another platform, or an empty one, is rejected rather than reinterpreted.
pub(crate) fn atspi_element(handle: &NativeHandle) -> Result<&AtspiElement, AdapterError> {
    handle.downcast_ref::<AtspiElement>().ok_or_else(|| {
        AdapterError::new(
            ErrorCode::InvalidArgs,
            "Native handle does not contain a Linux accessibility element",
        )
        .with_details(serde_json::json!({
            "kind": "invalid_native_handle",
            "platform": "linux",
            "empty": handle.is_null()
        }))
    })
}
