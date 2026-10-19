use agent_desktop_core::{AdapterError, DeliverySemantics};

#[derive(Debug, Default)]
pub(crate) struct ReleaseState {
    armed: bool,
    delivered: u32,
}

impl ReleaseState {
    pub(crate) fn resumed(delivered: u32, armed: bool) -> Self {
        Self { armed, delivered }
    }

    pub(crate) fn arm(&mut self) {
        self.armed = true;
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }

    pub(crate) fn mark_delivered(&mut self) {
        self.delivered = self.delivered.saturating_add(1);
    }

    pub(crate) fn delivered(&self) -> u32 {
        self.delivered
    }

    pub(crate) fn should_release(&self) -> bool {
        self.armed
    }

    pub(crate) fn enrich_error(&self, error: AdapterError, suggestion: &str) -> AdapterError {
        if self.delivered == 0 {
            return error.with_disposition(DeliverySemantics::not_delivered());
        }
        let mut details = error
            .details
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(map) = details.as_object_mut() {
            map.insert("delivered_events".into(), self.delivered.into());
            if self.armed {
                map.insert("emergency_release_posted".into(), true.into());
                map.insert("emergency_release_acknowledged".into(), false.into());
            }
        }
        error
            .with_disposition(DeliverySemantics::delivered_unverified())
            .with_details(details)
            .with_suggestion(suggestion)
    }
}

#[cfg(test)]
#[path = "release_state_tests.rs"]
mod tests;
