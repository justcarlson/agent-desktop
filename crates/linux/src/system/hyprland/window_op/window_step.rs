use super::client_state::ClientState;
use super::step_goal::StepGoal;

/// One dispatch request and what it must leave on the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowStep {
    pub(crate) command: String,
    pub(crate) goals: Vec<StepGoal>,
}

impl WindowStep {
    pub(crate) fn new(command: String, goals: Vec<StepGoal>) -> Self {
        Self { command, goals }
    }

    pub(crate) fn holds(&self, state: &ClientState) -> bool {
        self.goals.iter().all(|goal| goal.holds(state))
    }
}
