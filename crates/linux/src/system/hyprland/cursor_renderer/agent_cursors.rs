//! The renderer's cursors, one per agent. A session without agents has one
//! cursor under an empty key. A multi-agent session keys every cursor by its
//! agent id and refuses a control that names no agent, so an unnamed caller
//! never moves another agent's cursor. A new agent's cursor starts at the
//! renderer's home point, where the first cursor appeared.

use std::collections::BTreeMap;

use agent_desktop_core::{AdapterError, CursorOverlayControl, ErrorCode, Point};

use super::agent_cursor::AgentCursor;

/// The key of the one cursor of a session without agents.
pub(crate) const SOLE_CURSOR: &str = "";

/// Every cursor the renderer draws, keyed by agent.
pub(crate) struct AgentCursors {
    cursors: BTreeMap<String, AgentCursor>,
    home: Point,
}

impl AgentCursors {
    /// No cursors yet; new ones start at `home`.
    pub(crate) fn new(home: Point) -> Self {
        Self {
            cursors: BTreeMap::new(),
            home,
        }
    }

    /// The cursor key `control` addresses. `multi_agent` is asked only when
    /// the control names no agent; a multi-agent session then refuses it.
    pub(crate) fn key_for(
        control: &CursorOverlayControl,
        multi_agent: impl FnOnce() -> bool,
    ) -> Result<String, AdapterError> {
        match control.agent_id() {
            Some(agent) => Ok(agent.to_owned()),
            None if multi_agent() => Err(missing_agent_refusal()),
            None => Ok(SOLE_CURSOR.to_owned()),
        }
    }

    /// The cursor under `key`, created at home with `control`'s style and
    /// motion, or updated to them when it exists.
    pub(crate) fn cursor(&mut self, key: &str, control: &CursorOverlayControl) -> &mut AgentCursor {
        let home = &self.home;
        let cursor = self.cursors.entry(key.to_owned()).or_insert_with(|| {
            let mut made = AgentCursor::new(control);
            made.pose = Some(home.clone());
            made
        });
        cursor.adopt(control);
        cursor
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut AgentCursor> {
        self.cursors.get_mut(key)
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<AgentCursor> {
        self.cursors.remove(key)
    }

    pub(crate) fn len(&self) -> usize {
        self.cursors.len()
    }

    /// The agent ids with a cursor, without the sole cursor's empty key.
    pub(crate) fn agents(&self) -> Vec<&str> {
        self.cursors
            .keys()
            .map(String::as_str)
            .filter(|key| !key.is_empty())
            .collect()
    }

    /// Whether any cursor has a travel in flight.
    pub(crate) fn travelling(&self) -> bool {
        self.cursors.values().any(|cursor| cursor.travel.is_some())
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut AgentCursor)> {
        self.cursors.iter_mut()
    }
}

/// The refusal for a cursor control with no agent id in a multi-agent
/// session.
pub(crate) fn missing_agent_refusal() -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "A multi-agent session's cursor control must name its agent",
    )
    .with_suggestion("Set --agent-id or AGENT_DESKTOP_AGENT_ID")
}

#[cfg(test)]
#[path = "agent_cursors_tests.rs"]
mod tests;
