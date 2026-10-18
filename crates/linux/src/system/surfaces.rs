use agent_desktop_core::{AdapterError, Deadline, ErrorCode, ProcessIdentity, SurfaceInfo};

use super::SharedBus;
use super::listed_window::ListedWindow;
use super::process_facts;
use super::registry::read_registry;
use super::window_ops::windows_of;

fn require_same_process(process: &ProcessIdentity) -> Result<(), AdapterError> {
    let Some(facts) = process_facts::read(process.pid.get()) else {
        return Err(AdapterError::new(
            ErrorCode::AppNotFound,
            "The process is no longer running",
        ));
    };
    if facts.instance != process.instance {
        return Err(AdapterError::new(
            ErrorCode::AppNotFound,
            "The process id was reused by another process",
        )
        .with_suggestion("List the applications again"));
    }
    Ok(())
}

fn surface(id: &str, kind: &str, title: &Option<String>) -> SurfaceInfo {
    SurfaceInfo {
        id: id.to_owned(),
        kind: kind.to_owned(),
        title: title.clone(),
        item_count: None,
        unclassified: Vec::new(),
    }
}

/// The surfaces of one listed window: a `window` entry always, a `sheet`
/// entry alongside it when the window is a modal dialog.
pub(crate) fn surfaces_of(windows: &[ListedWindow]) -> Vec<SurfaceInfo> {
    windows
        .iter()
        .flat_map(|window| {
            let title = (!window.info.title.is_empty()).then(|| window.info.title.clone());
            let mut entries = vec![surface(&window.info.id, "window", &title)];
            if window.sheet {
                entries.push(surface(&window.info.id, "sheet", &title));
            }
            entries
        })
        .collect()
}

/// Lists the surfaces the named process presents. The process identity is
/// checked first so a surface list never answers for a pid that has since
/// become a different process.
pub(crate) fn list_surfaces(
    bus: &SharedBus,
    process: &ProcessIdentity,
    deadline: Deadline,
) -> Result<Vec<SurfaceInfo>, AdapterError> {
    require_same_process(process)?;
    let registry = read_registry(bus, deadline)?;
    let pid = process.pid.get();
    let windows = windows_of(bus, &registry, |app| app.root.pid() == pid, deadline)?;
    Ok(surfaces_of(&windows))
}

#[cfg(test)]
#[path = "surfaces_tests.rs"]
mod tests;
