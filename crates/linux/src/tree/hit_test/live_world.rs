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
use crate::tree::hit_descent::{BusNodes, Described, Hit, NodeSource};
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
    connection: &'a Connection,
    deadline: Deadline,
    nodes: BusNodes<'a>,
    popup: bool,
    popup_rect: Option<Extents>,
}

impl<'a> LiveWorld<'a> {
    pub(super) fn new(connection: &'a Connection, deadline: Deadline) -> Self {
        Self {
            connection,
            deadline,
            nodes: BusNodes {
                connection,
                deadline,
            },
            popup: false,
            popup_rect: None,
        }
    }
}

impl NodeSource for LiveWorld<'_> {
    fn at_point(&mut self, from: &AtspiElement, point: (i32, i32)) -> Result<Hit, AdapterError> {
        self.nodes.at_point(from, point)
    }

    fn verified_parent(
        &mut self,
        node: &AtspiElement,
    ) -> Result<Option<AtspiElement>, AdapterError> {
        self.nodes.verified_parent(node)
    }

    fn describe(&mut self, node: &AtspiElement) -> Result<Described, AdapterError> {
        self.nodes.describe(node)
    }
}

impl HitWorld for LiveWorld<'_> {
    fn facts(
        &mut self,
        target: &AtspiElement,
        frame: &AtspiElement,
    ) -> Result<Option<Facts>, AdapterError> {
        let root = AtspiElement::new(frame.bus_name(), REGISTRY_ROOT_PATH, frame.pid());
        let (screen, window, frame_node, toolkit) = within(self.deadline, async {
            Ok(join4(
                read_node(self.connection, target),
                read_window_extents_async(self.connection, target),
                read_node(self.connection, frame),
                read_toolkit(self.connection, &root),
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
            within(self.deadline, async {
                Ok(read_window_extents_async(self.connection, frame).await)
            })?
        } else {
            None
        };
        Ok(Some(Facts {
            screen,
            window,
            frame: FrameFacts {
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
        let state = DisplayReader.clients_within(self.deadline);
        let X11State::Ready { clients, .. } = state else {
            return None;
        };
        let (client, coordinate_origin) = if self.popup {
            let rect = self.popup_rect?;
            let popup =
                DisplayReader::popup_window(frame.pid, (rect.width, rect.height), self.deadline)?;
            let origin =
                crate::tree::frame_geometry::popup_origin((popup.bounds.x, popup.bounds.y), rect)?;
            (popup.id, Some(origin))
        } else {
            let ClientMatch::One(client) = match_client(frame, &clients) else {
                return None;
            };
            (client, None)
        };
        match DisplayReader.read_within(client, self.deadline) {
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
