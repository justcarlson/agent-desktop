use agent_desktop_core::{AdapterError, Deadline};
use futures_util::future::join4;
use zbus::Connection;

use super::{Facts, HitWorld, Windowing};
use crate::system::shell::container::MENU;
use crate::system::x11::map::{ClientMatch, match_client};
use crate::system::x11::stacking_read::StackingRead;
use crate::system::x11::stacking_source::StackingSource;
use crate::system::x11::{DisplayReader, FrameFacts, X11State};
use crate::system::{REGISTRY_ROOT_PATH, within};
use crate::tree::hit_descent::{BusNodes, NodeSource};
use crate::tree::resolve_errors::{stopped_app, stopped_aware};
use crate::tree::state_bits::{ICONIFIED, StateBits};
use crate::tree::toolkit_trust::{ToolkitKind, kind, read_toolkit};
use crate::tree::window_extents::read_window_extents_async;
use crate::tree::{AtspiElement, Extents, read_node};

pub(super) fn stopped_aware_for(error: AdapterError, frame: &AtspiElement) -> AdapterError {
    if error.code == agent_desktop_core::ErrorCode::Timeout {
        let stopped = stopped_app(frame.pid());
        stopped_aware(error, stopped.as_deref())
    } else {
        error
    }
}

/// The bus and the display server behind one hit test.
pub(super) struct LiveWorld<'a> {
    nodes: BusNodes<'a>,
    popup: bool,
    popup_rect: Option<Extents>,
}

impl<'a> LiveWorld<'a> {
    pub(super) fn new(connection: &'a Connection, deadline: Deadline) -> Self {
        Self {
            nodes: BusNodes {
                connection,
                deadline,
            },
            popup: false,
            popup_rect: None,
        }
    }
}

impl HitWorld for LiveWorld<'_> {
    fn nodes(&mut self) -> &mut impl NodeSource {
        &mut self.nodes
    }

    fn facts(
        &mut self,
        target: &AtspiElement,
        frame: &AtspiElement,
    ) -> Result<Option<Facts>, AdapterError> {
        let root = AtspiElement::new(frame.bus_name(), REGISTRY_ROOT_PATH, frame.pid());
        let connection = self.nodes.connection;
        let deadline = self.nodes.deadline;
        let (screen, window, frame_node, toolkit) = within(deadline, async {
            Ok(join4(
                read_node(connection, target),
                read_window_extents_async(connection, target),
                read_node(connection, frame),
                read_toolkit(connection, &root),
            )
            .await)
        })?;
        let (Ok(screen), Some(window), Ok(frame_node)) = (screen, window, frame_node) else {
            return Ok(None);
        };
        let (Some(screen), Some(states)) = (
            screen.extents.present().copied(),
            frame_node.traits.states.present(),
        ) else {
            return Ok(None);
        };
        let title = frame_node
            .properties
            .name
            .present()
            .cloned()
            .unwrap_or_default();
        let global_position = crate::tree::toolkit_trust::reports_global_position(&toolkit);
        self.popup =
            frame_node.traits.role.present() == Some(&MENU) && kind(&toolkit) == ToolkitKind::Gtk4;
        self.popup_rect = if self.popup {
            within(deadline, async {
                Ok(read_window_extents_async(connection, frame).await)
            })?
        } else {
            None
        };
        Ok(Some(Facts {
            screen,
            window,
            frame: FrameFacts {
                size_identity: false,
                pid: frame.pid(),
                title,
                extents: frame_node.extents.present().copied(),
                global_position,
            },
            minimized: StateBits::new(states).has(ICONIFIED),
            toolkit,
        }))
    }

    fn windowing(&mut self, frame: &FrameFacts) -> Option<Windowing> {
        let state = DisplayReader.clients_within(self.nodes.deadline);
        let X11State::Ready { clients, .. } = state else {
            return None;
        };
        let (client, coordinate_origin) = if self.popup {
            let rect = self.popup_rect?;
            let popup = DisplayReader::popup_window(
                frame.pid,
                (rect.width, rect.height),
                self.nodes.deadline,
            )?;
            let origin =
                crate::tree::frame_geometry::popup_origin((popup.bounds.x, popup.bounds.y), rect)?;
            (popup.id, Some(origin))
        } else {
            let ClientMatch::One(client) = match_client(frame, &clients) else {
                return None;
            };
            (client, None)
        };
        match DisplayReader.read_within(client, self.nodes.deadline) {
            StackingRead::Ready(snapshot) => Some(Windowing {
                coordinate_origin,
                client,
                snapshot,
                clients,
            }),
            StackingRead::NotX11 | StackingRead::NoManager | StackingRead::Unavailable => None,
        }
    }
}
