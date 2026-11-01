//! The renderer wire messages. One JSON object per connection, named by
//! `message`:
//!
//! - `enable`, `present`, `hide_cursor`, `show_cursor`, `disable` carry the
//!   core control as `control`; `present` adds `park`.
//! - `hide` carries `holder` and `expiry_ms`; `show` carries `holder`.
//! - `status` carries nothing.

use agent_desktop_core::{AdapterError, CursorOverlayControl, ErrorCode};
use serde_json::{Value, json};

use crate::system::hyprland::placement::hidden_placements;

/// One request to a renderer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RendererRequest {
    Control {
        control: Box<CursorOverlayControl>,
        park: bool,
    },
    Hide {
        holder: String,
        expiry_ms: u64,
    },
    Show {
        holder: String,
    },
    Status,
}

impl RendererRequest {
    /// The request for `control`. A `present` into a placement this
    /// command found hidden is parked.
    pub(crate) fn control(control: CursorOverlayControl) -> Self {
        let park = control.instruction().is_some_and(|instruction| {
            std::iter::once(instruction.destination())
                .chain(instruction.drag_from())
                .any(hidden_placements::hides)
        });
        Self::Control {
            control: Box::new(control),
            park,
        }
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Control { control, .. } => control_message(control),
            Self::Hide { .. } => "hide",
            Self::Show { .. } => "show",
            Self::Status => "status",
        }
    }

    pub(crate) fn to_json(&self) -> Result<Value, AdapterError> {
        let message = self.message();
        Ok(match self {
            Self::Control { control, park } => {
                let control = serde_json::to_value(control).map_err(encode_error)?;
                if control_message_is_present(message) {
                    json!({"message": message, "control": control, "park": park})
                } else {
                    json!({"message": message, "control": control})
                }
            }
            Self::Hide { holder, expiry_ms } => {
                json!({"message": message, "holder": holder, "expiry_ms": expiry_ms})
            }
            Self::Show { holder } => json!({"message": message, "holder": holder}),
            Self::Status => json!({"message": message}),
        })
    }

    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, AdapterError> {
        let value: Value = serde_json::from_slice(bytes).map_err(decode_error)?;
        let message = value
            .get("message")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("missing message"))?;
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| invalid(key))
        };
        match message {
            "hide" => Ok(Self::Hide {
                holder: text("holder")?,
                expiry_ms: value
                    .get("expiry_ms")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| invalid("expiry_ms"))?,
            }),
            "show" => Ok(Self::Show {
                holder: text("holder")?,
            }),
            "status" => Ok(Self::Status),
            _ => {
                let control: CursorOverlayControl = value
                    .get("control")
                    .cloned()
                    .map(serde_json::from_value)
                    .ok_or_else(|| invalid("control"))?
                    .map_err(decode_error)?;
                if control_message(&control) != message {
                    return Err(invalid("message does not match its control"));
                }
                control.validate()?;
                let park = value.get("park").and_then(Value::as_bool).unwrap_or(false);
                Ok(Self::Control {
                    control: Box::new(control),
                    park,
                })
            }
        }
    }
}

fn control_message(control: &CursorOverlayControl) -> &'static str {
    match control {
        CursorOverlayControl::Enable { .. } => "enable",
        CursorOverlayControl::Present { .. } => "present",
        CursorOverlayControl::Hide { .. } => "hide_cursor",
        CursorOverlayControl::Show { .. } => "show_cursor",
        CursorOverlayControl::Disable { .. } => "disable",
    }
}

fn control_message_is_present(message: &str) -> bool {
    message == "present"
}

fn encode_error(error: serde_json::Error) -> AdapterError {
    AdapterError::internal("Could not encode the cursor renderer request")
        .with_platform_detail(error.to_string())
}

fn decode_error(error: serde_json::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "Could not decode the cursor renderer request",
    )
    .with_platform_detail(error.to_string())
}

fn invalid(what: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        format!("Invalid cursor renderer request: {what}"),
    )
}

#[cfg(test)]
#[path = "renderer_request_tests.rs"]
mod tests;
