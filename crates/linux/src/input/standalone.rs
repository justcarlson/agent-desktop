use agent_desktop_core::AdapterError;

pub(crate) fn reject_standalone(what: &'static str) -> AdapterError {
    AdapterError::not_supported(what)
}

#[cfg(test)]
#[path = "standalone_tests.rs"]
mod tests;
