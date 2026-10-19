use agent_desktop_core::Deadline;

use super::AtspiElement;
use super::live_apps::{qt, terminal};
use super::live_rig::rig;
use super::toolkit_trust::{ToolkitKind, kind};
use crate::LinuxAdapter;
use crate::system::registry::toolkit;
use crate::system::{REGISTRY_ROOT_PATH, SharedBus, within};

fn registry_kind(bus: &SharedBus, frame: &AtspiElement) -> Result<ToolkitKind, String> {
    let deadline = Deadline::standard().map_err(|error| error.message)?;
    let connection = bus.get(deadline).map_err(|error| error.message)?;
    let root = AtspiElement::new(frame.bus_name(), REGISTRY_ROOT_PATH, frame.pid());
    let id = within(deadline, async {
        Ok(toolkit(connection.inner(), &root).await)
    })
    .map_err(|error| error.message)?;
    Ok(kind(&id))
}

fn expect_kind(
    wanted: ToolkitKind,
) -> impl Fn(&str, &LinuxAdapter, &SharedBus, &AtspiElement) -> Result<(), String> {
    move |label, _, bus, frame| {
        let found = registry_kind(bus, frame)?;
        if found == wanted {
            Ok(())
        } else {
            Err(format!(
                "[{label}] registry toolkit read {found:?}, wanted {wanted:?}"
            ))
        }
    }
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn registry_toolkit_live_reads_gtk3_and_qt_that_refuse_the_group_read() {
    let session = rig();
    terminal(
        &session.adapter,
        &session.bus,
        |label, adapter, bus, frame| expect_kind(ToolkitKind::Gtk3)(label, adapter, bus, frame),
    )
    .expect("gtk3 terminal");
    qt(
        &session.adapter,
        &session.bus,
        |label, adapter, bus, frame| expect_kind(ToolkitKind::Qt)(label, adapter, bus, frame),
    )
    .expect("qt");
}
