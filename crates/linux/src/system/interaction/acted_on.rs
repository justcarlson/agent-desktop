use std::sync::Mutex;

use agent_desktop_core::{ActionResult, AdapterError, DeliverySemantics};

use crate::tree::AtspiElement;

/// The element the last delivered action reached. A popup item has no screen
/// position once its popup closes, so a read of it after the action that
/// closed the popup must not be refused for the missing position: the
/// refusal guards the aim of a pointer, and the aim is over.
#[derive(Default)]
pub(crate) struct ActedOn {
    element: Mutex<Option<AtspiElement>>,
}

impl ActedOn {
    pub(crate) fn record(
        &self,
        element: Option<&AtspiElement>,
        result: &Result<ActionResult, AdapterError>,
    ) {
        let disposition = match result {
            Ok(action) => action.disposition(),
            Err(error) => error.disposition,
        };
        if disposition == DeliverySemantics::not_delivered() {
            return;
        }
        if let Ok(mut acted) = self.element.lock() {
            *acted = element.cloned();
        }
    }

    pub(crate) fn covers(&self, element: &AtspiElement) -> bool {
        self.element
            .lock()
            .is_ok_and(|acted| acted.as_ref() == Some(element))
    }
}

#[cfg(test)]
#[path = "acted_on_tests.rs"]
mod tests;
