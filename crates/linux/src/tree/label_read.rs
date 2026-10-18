use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::node_read::{NAME, call, get_one, string_value};
use super::{AtspiElement, Field, LabelOutcome};
use crate::system::ACCESSIBLE_INTERFACE;
use agent_desktop_core::AdapterError;

const LABELLED_BY: u32 = 2;

type RelationSet = Vec<(u32, Vec<(String, OwnedObjectPath)>)>;

pub(super) async fn first_label_target(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<Option<AtspiElement>, zbus::Error> {
    let relations = call(
        connection,
        element,
        ACCESSIBLE_INTERFACE,
        "GetRelationSet",
        &(),
    )
    .await?
    .body()
    .deserialize::<RelationSet>()?;
    Ok(relations
        .into_iter()
        .find(|(kind, _)| *kind == LABELLED_BY)
        .and_then(|(_, targets)| targets.into_iter().next())
        .map(|(bus, path)| AtspiElement::new(bus, path.to_string(), element.pid())))
}

async fn name_of(
    connection: &Connection,
    target: &AtspiElement,
) -> Result<Field<String>, AdapterError> {
    get_one(connection, target, NAME, string_value).await
}

/// Reads the text of the node's `LabelledBy` target. Meant for a node whose
/// own `Name` is blank: it costs one relation read and one name read.
#[allow(dead_code)]
pub(crate) async fn read_label(connection: &Connection, element: &AtspiElement) -> LabelOutcome {
    let target = match first_label_target(connection, element).await {
        Ok(Some(target)) => target,
        Ok(None) => return LabelOutcome::Unlabelled,
        Err(_) => return LabelOutcome::Failed,
    };
    match name_of(connection, &target).await {
        Ok(Field::Failed(_)) | Err(_) => LabelOutcome::Failed,
        Ok(field) => field.non_blank().map_or(LabelOutcome::Unlabelled, |text| {
            LabelOutcome::Text(text.clone())
        }),
    }
}
