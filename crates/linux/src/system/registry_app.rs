use serde_json::{Value, json};

use super::app_name::{app_name, name_matches};
use super::process_facts::ProcessFacts;
use super::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;

/// One application on the accessibility registry. `answered` is false when
/// the application's own root did not reply in time; it is still an
/// application, named from `/proc` like every other. `registry_name` is the
/// root's own `Name` while it answered, kept only so a query by that name
/// still finds the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryApp {
    pub(crate) root: AtspiElement,
    pub(crate) name: String,
    pub(crate) registry_name: Option<String>,
    pub(crate) answered: bool,
    pub(crate) facts: ProcessFacts,
    pub(crate) toolkit: ToolkitId,
}

/// Builds the entry for an application from what its bus name, its process
/// and its root said. `root_name` is `None` when the root did not answer.
/// Nothing the root failed to say removes the application from the list.
pub(crate) fn assemble(
    root: AtspiElement,
    facts: ProcessFacts,
    root_name: Option<Option<String>>,
    toolkit: ToolkitId,
) -> RegistryApp {
    let answered = root_name.is_some();
    let name = app_name(&facts);
    let registry_name = root_name.flatten().filter(|text| !text.trim().is_empty());
    RegistryApp {
        root,
        name,
        registry_name,
        answered,
        facts,
        toolkit,
    }
}

impl RegistryApp {
    /// Whether `query` names this application.
    pub(crate) fn matches(&self, query: &str) -> bool {
        name_matches(
            query,
            &self.name,
            self.registry_name.as_deref(),
            &self.facts,
        )
    }
}

pub(crate) fn skipped(bus: &str, pid: Option<u32>, reason: &str) -> Value {
    let mut record = json!({ "bus": bus, "reason": reason });
    if let Some(pid) = pid {
        record["pid"] = json!(pid);
    }
    record
}

#[cfg(test)]
#[path = "registry_app_tests.rs"]
mod tests;
