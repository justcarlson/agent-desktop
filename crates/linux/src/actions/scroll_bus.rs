use agent_desktop_core::{AdapterError, Deadline, ErrorCode, NativeHandle, state};
use futures_util::future::join3;
use zbus::Connection;
use zbus::zvariant::{OwnedValue, Value};

use super::scroll_crawl::{Crawl, orientation_of, role_of};
use super::scroll_find_bar::{Site, choose_bar};
use super::scroll_into_view::{BarFound, Geometry, ScrollPort, View};
use super::scroll_plan::{Axis, BarRange};
use crate::system::toolkit_id::ToolkitId;
use crate::system::{PROPERTIES_INTERFACE, SharedBus, classify, within};
use crate::tree::live_offscreen::toolkit_of;
use crate::tree::role_numbers::SCROLL_BAR;
use crate::tree::window_extents::read_window_extents;
use crate::tree::{
    AtspiElement, COMPONENT_INTERFACE, Extents, get_element_bounds, get_live_state, list_children,
};

const VALUE_INTERFACE: &str = "org.a11y.atspi.Value";
const TOP_LEFT: u32 = 0;

/// The accessibility-bus side of a scroll: reads the target, calls
/// `ScrollTo`, finds the scroll bar and writes its `Value`.
pub(crate) struct BusPort<'a> {
    bus: &'a SharedBus,
    handle: &'a NativeHandle,
    element: &'a AtspiElement,
    deadline: Deadline,
    bar: Option<AtspiElement>,
    crawl: Option<Crawl>,
}

impl<'a> BusPort<'a> {
    pub(crate) fn new(
        bus: &'a SharedBus,
        handle: &'a NativeHandle,
        element: &'a AtspiElement,
        deadline: Deadline,
    ) -> Self {
        Self {
            bus,
            handle,
            element,
            deadline,
            bar: None,
            crawl: None,
        }
    }

    fn connection(&self) -> Result<zbus::blocking::Connection, AdapterError> {
        self.bus.get(self.deadline)
    }

    fn crawl(&mut self) -> Result<Option<&Crawl>, AdapterError> {
        if self.crawl.is_none() {
            let Some(frame) = self.element.frame() else {
                return Ok(None);
            };
            self.crawl = Some(Crawl::of(self.bus, frame, self.element, self.deadline)?);
        }
        Ok(self.crawl.as_ref())
    }

    fn neighbour(&mut self) -> Result<Option<(i64, Extents)>, AdapterError> {
        let (bus, deadline) = (self.bus, self.deadline);
        let Some(crawl) = self.crawl()? else {
            return Ok(None);
        };
        let Some(target) = crawl.target else {
            return Ok(None);
        };
        let Some(parent) = crawl.nodes[target].1 else {
            return Ok(None);
        };
        let siblings: Vec<usize> = crawl
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.1 == Some(parent))
            .map(|(index, _)| index)
            .collect();
        let place = siblings.iter().position(|&index| index == target);
        let Some(place) = place else {
            return Ok(None);
        };
        let mut order: Vec<usize> = (0..siblings.len()).filter(|&at| at != place).collect();
        order.sort_by_key(|&at| at.abs_diff(place));
        for at in order {
            if deadline.is_expired() {
                break;
            }
            let rect = read_window_extents(bus, &crawl.nodes[siblings[at]].0, deadline);
            if let Some(rect) = rect.filter(|rect| positive(*rect)) {
                return Ok(Some((place as i64 - at as i64, rect)));
            }
        }
        Ok(None)
    }

    fn bar_in_container(&self, axis: Axis) -> Result<Option<AtspiElement>, AdapterError> {
        let Some(container) = self.element.scroll_container() else {
            return Ok(None);
        };
        let connection = self.connection()?;
        let bars = within(self.deadline, async {
            let children = list_children(connection.inner(), container).await?;
            let mut found = Vec::new();
            for (bus, path) in children {
                let child = AtspiElement::new(bus, path, container.pid());
                if role_is_bar(connection.inner(), &child).await {
                    let way = orientation_of(connection.inner(), &child).await;
                    found.push((child, way));
                }
            }
            Ok(found)
        })?;
        let mut sites = vec![Site {
            parent: None,
            bar: None,
        }];
        sites.extend(bars.iter().map(|(_, way)| Site {
            parent: Some(0),
            bar: Some(*way),
        }));
        Ok(choose_bar(&sites, None, axis)
            .and_then(|index| bars.into_iter().nth(index - 1))
            .map(|(bar, _)| bar))
    }
}

fn positive(rect: Extents) -> bool {
    rect.width > 0 && rect.height > 0 && rect.x != i32::MIN && rect.y != i32::MIN
}

async fn role_is_bar(connection: &Connection, element: &AtspiElement) -> bool {
    role_of(connection, element).await == SCROLL_BAR
}

async fn number(connection: &Connection, bar: &AtspiElement, name: &str) -> Option<f64> {
    let reply = connection
        .call_method(
            Some(bar.bus_name()),
            bar.path(),
            Some(PROPERTIES_INTERFACE),
            "Get",
            &(VALUE_INTERFACE, name),
        )
        .await
        .ok()?;
    let value = reply.body().deserialize::<OwnedValue>().ok()?;
    value
        .downcast_ref::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

impl ScrollPort for BusPort<'_> {
    fn toolkit(&mut self) -> ToolkitId {
        toolkit_of(self.bus, self.element, self.deadline)
    }

    fn view(&mut self) -> Result<View, AdapterError> {
        let Some(read) = get_live_state(self.bus, self.handle, self.deadline)? else {
            return Ok(View {
                offscreen: None,
                hidden: false,
                bounds_provable: false,
            });
        };
        let bounds_provable = read.offscreen == Some(true)
            || get_element_bounds(self.bus, self.handle, self.deadline)?.is_some();
        Ok(View {
            offscreen: read.offscreen,
            hidden: read.states.iter().any(|token| token == state::HIDDEN),
            bounds_provable,
        })
    }

    fn observe(&mut self) -> Result<Option<bool>, AdapterError> {
        match get_live_state(self.bus, self.handle, self.deadline) {
            Ok(read) => Ok(read.and_then(|read| read.offscreen)),
            Err(error) if error.code == ErrorCode::Timeout => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn scroll_to(&mut self) -> Result<bool, AdapterError> {
        let connection = self.connection()?;
        let element = self.element;
        within(self.deadline, async {
            let reply = connection
                .inner()
                .call_method(
                    Some(element.bus_name()),
                    element.path(),
                    Some(COMPONENT_INTERFACE),
                    "ScrollTo",
                    &(TOP_LEFT,),
                )
                .await;
            match reply {
                Ok(message) => message
                    .body()
                    .deserialize::<bool>()
                    .map_err(|error| classify(&error)),
                Err(zbus::Error::MethodError(..)) => Ok(false),
                Err(error) => Err(classify(&error)),
            }
        })
    }

    fn geometry(&mut self) -> Result<Geometry, AdapterError> {
        let (bus, deadline) = (self.bus, self.deadline);
        let target = read_window_extents(bus, self.element, deadline);
        let positioned = target.is_some_and(positive);
        Ok(Geometry {
            target,
            container: self
                .element
                .scroll_container()
                .and_then(|container| read_window_extents(bus, container, deadline)),
            frame: self
                .element
                .frame()
                .and_then(|frame| read_window_extents(bus, frame, deadline)),
            neighbour: if positioned { None } else { self.neighbour()? },
        })
    }

    fn find_bar(&mut self, axis: Axis) -> Result<BarFound, AdapterError> {
        let has_container = self.element.scroll_container().is_some();
        let mut found = self.bar_in_container(axis)?;
        if found.is_none() {
            let (bus, deadline) = (self.bus, self.deadline);
            if let Some(crawl) = self.crawl()?.filter(|crawl| crawl.target.is_some()) {
                let sites = crawl.sites(bus, deadline)?;
                found = choose_bar(&sites, crawl.target, axis)
                    .map(|index| crawl.nodes[index].0.clone());
            }
        }
        let Some(bar) = found else {
            return Ok(BarFound::Missing {
                container: has_container,
            });
        };
        let extents = read_window_extents(self.bus, &bar, self.deadline);
        self.bar = Some(bar);
        Ok(BarFound::Found { extents })
    }

    fn range(&mut self) -> Result<Option<BarRange>, AdapterError> {
        let Some(bar) = self.bar.as_ref() else {
            return Ok(None);
        };
        let connection = self.connection()?;
        let (min, max, current) = within(self.deadline, async {
            Ok(join3(
                number(connection.inner(), bar, "MinimumValue"),
                number(connection.inner(), bar, "MaximumValue"),
                number(connection.inner(), bar, "CurrentValue"),
            )
            .await)
        })?;
        Ok(match (min, max, current) {
            (Some(min), Some(max), Some(current)) if max > min => {
                Some(BarRange { min, max, current })
            }
            _ => None,
        })
    }

    fn write(&mut self, value: f64) -> Result<(), AdapterError> {
        let Some(bar) = self.bar.as_ref() else {
            return Err(AdapterError::new(
                ErrorCode::Internal,
                "No scroll bar was found to write",
            ));
        };
        let connection = self.connection()?;
        within(self.deadline, async {
            connection
                .inner()
                .call_method(
                    Some(bar.bus_name()),
                    bar.path(),
                    Some(PROPERTIES_INTERFACE),
                    "Set",
                    &(VALUE_INTERFACE, "CurrentValue", Value::from(value)),
                )
                .await
                .map(|_| ())
                .map_err(|error| classify(&error))
        })
    }
}
