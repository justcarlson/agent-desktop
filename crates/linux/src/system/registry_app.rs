use serde_json::{Value, json};

use super::app_name::app_name;
use super::process_facts::ProcessFacts;
use super::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;

/// One application on the accessibility registry. `answered` is false when
/// the application's own root did not reply in time; it is still an
/// application, named from `/proc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryApp {
    pub(crate) root: AtspiElement,
    pub(crate) name: String,
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
    let name = app_name(root_name.flatten().as_deref(), &facts);
    RegistryApp {
        root,
        name,
        answered,
        facts,
        toolkit,
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
