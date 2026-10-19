use futures_util::future::join;
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use super::AtspiElement;
use super::node_read::call;
use super::text_units::LengthUnit;
use crate::system::PROPERTIES_INTERFACE;
use crate::system::toolkit_id::ToolkitId;

const APPLICATION_INTERFACE: &str = "org.a11y.atspi.Application";

async fn application_text(
    connection: &Connection,
    root: &AtspiElement,
    property: &'static str,
) -> Option<String> {
    let reply = call(
        connection,
        root,
        PROPERTIES_INTERFACE,
        "Get",
        &(APPLICATION_INTERFACE, property),
    )
    .await
    .ok()?;
    let value = reply.body().deserialize::<OwnedValue>().ok()?;
    value.downcast_ref::<&str>().ok().map(str::to_owned)
}

/// The toolkit an application root names, read one property at a time: GTK 3
/// and Qt both refuse `GetAll` on the application interface while answering
/// each property, so a group read would leave the toolkit unknown.
pub(crate) async fn read_toolkit(connection: &Connection, root: &AtspiElement) -> ToolkitId {
    let (name, version) = join(
        application_text(connection, root, "ToolkitName"),
        application_text(connection, root, "Version"),
    )
    .await;
    ToolkitId { name, version }
}

/// The toolkits whose accessibility geometry has been measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolkitKind {
    Gtk3,
    Gtk4,
    Qt,
    Chromium,
    Other,
}

fn gtk_major(version: Option<&str>) -> Option<u32> {
    version?.split('.').next()?.parse().ok()
}

/// The toolkit a root names. A GTK root with no readable major version is not
/// guessed at, because the major decides which coordinate model applies.
pub(crate) fn kind(toolkit: &ToolkitId) -> ToolkitKind {
    let Some(name) = toolkit.name.as_deref() else {
        return ToolkitKind::Other;
    };
    if name.eq_ignore_ascii_case("gtk") {
        return match gtk_major(toolkit.version.as_deref()) {
            Some(major) if major < 4 => ToolkitKind::Gtk3,
            Some(_) => ToolkitKind::Gtk4,
            None => ToolkitKind::Other,
        };
    }
    if name.eq_ignore_ascii_case("qt") {
        ToolkitKind::Qt
    } else if name.eq_ignore_ascii_case("chromium") {
        ToolkitKind::Chromium
    } else {
        ToolkitKind::Other
    }
}

/// The unit the toolkit counts the `InsertText` length in: Qt in UTF-16
/// units, every other toolkit in bytes (L30-38).
pub(crate) fn insert_length_unit(toolkit: &ToolkitId) -> LengthUnit {
    match kind(toolkit) {
        ToolkitKind::Qt => LengthUnit::Utf16Units,
        _ => LengthUnit::Utf8Bytes,
    }
}

/// Whether the toolkit reports screen positions that are global: GTK 3, Qt and
/// Chromium. GTK 4 reports `[0, 0, w, h]` for nearly every node, and a toolkit
/// that could not be read is not assumed to report anything.
pub(crate) fn reports_global_position(toolkit: &ToolkitId) -> bool {
    matches!(
        kind(toolkit),
        ToolkitKind::Gtk3 | ToolkitKind::Qt | ToolkitKind::Chromium
    )
}

/// Whether a descent in window coordinates lands on the node under the point
/// often enough to say a landing on the target means the target is reached.
/// GTK 4 qualifies in window coordinates only.
pub(crate) fn trusts_descent(toolkit: &ToolkitId) -> bool {
    kind(toolkit) != ToolkitKind::Other
}

/// Whether a landing on a node outside the target's ancestry in the same
/// window is evidence that something covers the target. That needs a measured
/// self-hit rate of 100% at the centres of visible nodes: GTK 3 and Qt reach
/// it; GTK 4 (3 of 9 on Text Editor) and Chromium, whose Electron hosts land
/// on unrelated nodes for 60 of 60, do not.
pub(crate) fn trusts_same_window_landing(toolkit: &ToolkitId) -> bool {
    matches!(kind(toolkit), ToolkitKind::Gtk3 | ToolkitKind::Qt)
}

#[cfg(test)]
#[path = "toolkit_trust_tests.rs"]
mod tests;
