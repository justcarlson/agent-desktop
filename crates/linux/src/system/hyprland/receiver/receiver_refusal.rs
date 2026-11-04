use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

/// Why receiver evidence refused a ref pointer action before any event.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReceiverRefusal {
    EvidenceMissing { check: &'static str },
    TargetNotShown,
    PointOutsideTarget,
    LayerCover { namespace: String, level: u8 },
    WindowCover { address: u64 },
}

impl ReceiverRefusal {
    fn cause(&self) -> &'static str {
        match self {
            Self::EvidenceMissing { .. } => "evidence_missing",
            Self::TargetNotShown => "target_not_shown",
            Self::PointOutsideTarget => "point_outside_target",
            Self::LayerCover { .. } => "layer_cover",
            Self::WindowCover { .. } => "window_cover",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::EvidenceMissing { check } => {
                format!("Hyprland gave no {check} evidence for the pointer target")
            }
            Self::TargetNotShown => "the target window is not shown on any output".to_owned(),
            Self::PointOutsideTarget => "the pointer target lies outside its window".to_owned(),
            Self::LayerCover { namespace, level } => {
                format!("layer surface '{namespace}' at level {level} covers the pointer target")
            }
            Self::WindowCover { address } => {
                format!("window 0x{address:x} covers the pointer target")
            }
        }
    }

    /// The not-delivered error this refusal returns before any event.
    pub(crate) fn to_error(&self) -> AdapterError {
        let mut details = json!({
            "kind": "receiver_evidence",
            "check": "hyprland_stack",
            "cause": self.cause(),
            "physical_delivery_started": false,
        });
        match self {
            Self::EvidenceMissing { check } => details["evidence"] = json!(check),
            Self::LayerCover { namespace, level } => {
                details["layer_namespace"] = json!(namespace);
                details["layer_level"] = json!(level);
            }
            Self::WindowCover { address } => {
                details["cover_address"] = json!(format!("0x{address:x}"))
            }
            Self::TargetNotShown | Self::PointOutsideTarget => {}
        }
        AdapterError::new(ErrorCode::ActionFailed, self.message())
            .with_details(details)
            .with_disposition(DeliverySemantics::not_delivered())
            .with_suggestion(
                "Use a ref action that does not need pointer input, or bring the target to the front and take a fresh snapshot",
            )
    }
}
