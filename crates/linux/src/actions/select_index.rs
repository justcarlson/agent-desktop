use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::context::Target;
use super::mutation::{WriteKind, WriteReply};
use super::write_call::{Call, invoke};
use crate::system::{ACCESSIBLE_INTERFACE, within};
use crate::tree::AtspiElement;
use crate::tree::toolkit_trust::{ToolkitKind, is_gecko, kind};

pub(super) const UNPROVEN_INDEX_KIND: &str = "selection_index_unproven";

fn supports_indices(toolkit: &crate::system::process::toolkit_id::ToolkitId) -> bool {
    match kind(toolkit) {
        ToolkitKind::Gtk4 => false,
        ToolkitKind::Other => is_gecko(toolkit),
        _ => true,
    }
}

pub(crate) trait IndexLookup {
    fn accessible_indices(&self) -> bool;
    fn index(&self, item: &AtspiElement) -> Result<i32, AdapterError>;
    fn child(&self, index: i32) -> Result<(String, String), AdapterError>;
    fn select(&self, index: i32) -> Result<WriteReply, AdapterError>;
}

impl IndexLookup for Target<'_> {
    fn select(&self, index: i32) -> Result<WriteReply, AdapterError> {
        invoke(
            self,
            Call {
                interface: super::select_exclusive::SELECTION_INTERFACE,
                method: "SelectChild",
                body: &(index,),
                kind: WriteKind::SelectChild,
            },
        )
    }

    fn accessible_indices(&self) -> bool {
        let toolkit =
            crate::tree::live_offscreen::toolkit_of(self.bus, self.element, self.deadline);
        supports_indices(&toolkit)
    }

    fn index(&self, item: &AtspiElement) -> Result<i32, AdapterError> {
        let connection = self.bus.get(self.deadline)?;
        within(self.deadline, async {
            let reply = connection
                .inner()
                .call_method(
                    Some(item.bus_name()),
                    item.path(),
                    Some(ACCESSIBLE_INTERFACE),
                    "GetIndexInParent",
                    &(),
                )
                .await
                .map_err(|error| crate::system::classify(&error))?;
            reply
                .body()
                .deserialize()
                .map_err(|error| AdapterError::internal(error.to_string()))
        })
    }

    fn child(&self, index: i32) -> Result<(String, String), AdapterError> {
        let connection = self.bus.get(self.deadline)?;
        within(self.deadline, async {
            let reply = connection
                .inner()
                .call_method(
                    Some(self.element.bus_name()),
                    self.element.path(),
                    Some(ACCESSIBLE_INTERFACE),
                    "GetChildAtIndex",
                    &(index,),
                )
                .await
                .map_err(|error| crate::system::classify(&error))?;
            let (bus, path): (String, zbus::zvariant::OwnedObjectPath) = reply
                .body()
                .deserialize()
                .map_err(|error| AdapterError::internal(error.to_string()))?;
            Ok((bus, path.to_string()))
        })
    }
}

pub(crate) fn write(
    lookup: &impl IndexLookup,
    item: &AtspiElement,
) -> Result<WriteReply, AdapterError> {
    lookup.select(resolve(lookup, item)?)
}

pub(crate) fn resolve(lookup: &impl IndexLookup, item: &AtspiElement) -> Result<i32, AdapterError> {
    if !lookup.accessible_indices() {
        return Err(AdapterError::new(
            ErrorCode::ActionNotSupported,
            "The container's selection indices cannot be mapped to accessible rows safely",
        )
        .with_suggestion("On X11, run select again with --headed: the pointer clicks the row and the result is checked against the row's selected state. A headless select cannot establish a safe index mapping, so refreshing or retrying it does not help.")
        .with_details(serde_json::json!({ "kind": UNPROVEN_INDEX_KIND, "retryable": false }))
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    let index = lookup.index(item)?;
    if index >= 0 {
        let child = lookup.child(index)?;
        if child.0 == item.bus_name() && child.1 == item.path() {
            return Ok(index);
        }
    }
    Err(AdapterError::new(
        ErrorCode::StaleRef,
        "The selection index no longer names the intended row",
    )
    .with_disposition(DeliverySemantics::not_delivered()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rows {
        index: i32,
        occupant: &'static str,
        toolkit: &'static str,
        writes: std::cell::Cell<usize>,
    }

    impl IndexLookup for Rows {
        fn select(&self, index: i32) -> Result<WriteReply, AdapterError> {
            assert_eq!(index, self.index);
            self.writes.set(self.writes.get() + 1);
            Ok(WriteReply::Done)
        }
        fn accessible_indices(&self) -> bool {
            supports_indices(&crate::system::process::toolkit_id::ToolkitId {
                name: Some(self.toolkit.into()),
                version: Some("4.14.5".into()),
            })
        }

        fn index(&self, _: &AtspiElement) -> Result<i32, AdapterError> {
            assert!(self.accessible_indices());
            Ok(self.index)
        }
        fn child(&self, index: i32) -> Result<(String, String), AdapterError> {
            assert_eq!(index, self.index);
            Ok((":1.2".into(), self.occupant.into()))
        }
    }

    #[test]
    fn selection_writes_only_the_revalidated_row_and_refuses_gtk4_indices() {
        let item = AtspiElement::new(":1.2", "/mouse", 7);
        let valid = Rows {
            index: 2,
            occupant: "/mouse",
            toolkit: "Qt",
            writes: Default::default(),
        };
        assert_eq!(write(&valid, &item).unwrap(), WriteReply::Done);
        assert_eq!(valid.writes.get(), 1);
        let replaced = Rows {
            index: 2,
            occupant: "/appearance",
            toolkit: "Qt",
            writes: Default::default(),
        };
        let error = write(&replaced, &item).unwrap_err();
        assert_eq!(error.code, ErrorCode::StaleRef);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert_eq!(replaced.writes.get(), 0);
        let gtk = Rows {
            index: 2,
            occupant: "/mouse",
            toolkit: "GTK",
            writes: Default::default(),
        };
        let error = write(&gtk, &item).unwrap_err();
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        let details = error.details.unwrap();
        assert_eq!(details["kind"], "selection_index_unproven");
        assert_eq!(details["retryable"], false);
        assert_eq!(gtk.writes.get(), 0);
    }
}
