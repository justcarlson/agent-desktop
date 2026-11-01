use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

const SEMANTIC_SUGGESTION: &str =
    "Use the semantic action on the element instead of keyboard input";

pub(crate) fn xkb_unavailable(detail: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The keyboard layout of this Wayland session could not be read",
    )
    .with_platform_detail(detail.to_owned())
    .with_suggestion("Install libxkbcommon, or use the semantic action on the element")
    .with_details(json!({ "physical_delivery_started": false, "cause": "xkb_unavailable" }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// A keymap the delivery cannot build or use. No key was sent.
pub(crate) fn keymap_refused(message: &str, cause: &str, extra: Value) -> AdapterError {
    let mut details = json!({ "physical_delivery_started": false, "cause": cause });
    if let (Some(map), Some(fields)) = (details.as_object_mut(), extra.as_object()) {
        map.extend(fields.clone());
    }
    AdapterError::new(ErrorCode::ActionNotSupported, message)
        .with_suggestion(SEMANTIC_SUGGESTION)
        .with_details(details)
        .with_disposition(DeliverySemantics::not_delivered())
}
