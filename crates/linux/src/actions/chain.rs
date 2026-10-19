use agent_desktop_core::{ActionResult, AdapterError, Deadline};

#[expect(dead_code, reason = "no caller yet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Observed {
    Matched,
    Unchanged,
    Unread,
}

#[expect(dead_code, reason = "no caller yet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryOutcome {
    NotDelivered,
    Satisfied,
    Delivered(Observed),
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) struct ChainDef {
    pub(crate) action: &'static str,
    pub(crate) suggestion: &'static str,
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) struct Rung<'a> {
    pub(crate) label: &'static str,
    pub(crate) run: Box<dyn FnMut() -> Result<DeliveryOutcome, AdapterError> + 'a>,
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn execute_chain(
    def: &ChainDef,
    _rungs: &mut [Rung<'_>],
    _deadline: Deadline,
) -> Result<ActionResult, AdapterError> {
    Err(AdapterError::not_supported(def.action))
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
