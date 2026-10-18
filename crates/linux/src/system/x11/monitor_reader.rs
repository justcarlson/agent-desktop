use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as RandrExt;
use x11rb::protocol::xproto::ConnectionExt as XprotoExt;
use x11rb::rust_connection::RustConnection;

use super::{MonitorSource, X11Monitor};
use crate::system::display_server::{DisplayServer, session_server};
use crate::tree::Extents;

/// Reads the RandR monitor list off the display named by `DISPLAY`, one
/// connection per call, like the window-manager reader. A display that
/// cannot be connected or whose replies do not decode answers `None`: the
/// list is unavailable, never partially reported.
pub(crate) struct MonitorReader;

fn output_name(
    connection: &RustConnection,
    output: x11rb::protocol::randr::Output,
) -> Option<String> {
    let info = connection
        .randr_get_output_info(output, 0)
        .ok()?
        .reply()
        .ok()?;
    String::from_utf8(info.name).ok()
}

fn monitors(display: &str) -> Option<Vec<X11Monitor>> {
    let (connection, screen) = RustConnection::connect(Some(display)).ok()?;
    let root = connection.setup().roots.get(screen)?.root;
    let reply = connection
        .randr_get_monitors(root, true)
        .ok()?
        .reply()
        .ok()?;
    let mut monitors = Vec::with_capacity(reply.monitors.len());
    for monitor in reply.monitors {
        let name = connection.get_atom_name(monitor.name).ok()?.reply().ok()?;
        let name = String::from_utf8(name.name).ok()?;
        let mut connectors = vec![name.clone()];
        for output in monitor.outputs {
            let Some(connector) = output_name(&connection, output) else {
                continue;
            };
            if !connectors.contains(&connector) {
                connectors.push(connector);
            }
        }
        monitors.push(X11Monitor {
            name,
            connectors,
            bounds: Extents {
                x: i32::from(monitor.x),
                y: i32::from(monitor.y),
                width: i32::from(monitor.width),
                height: i32::from(monitor.height),
            },
            primary: monitor.primary,
        });
    }
    Some(monitors)
}

impl MonitorSource for MonitorReader {
    fn read(&self) -> Option<Vec<X11Monitor>> {
        match session_server() {
            DisplayServer::X11(display) => monitors(&display),
            _ => None,
        }
    }
}
